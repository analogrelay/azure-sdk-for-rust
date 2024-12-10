// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::http::{headers::Headers, StatusCode};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::future::{Future, IntoFuture};
use std::{fmt, pin::Pin};
use typespec::error::{ErrorKind, ResultExt};

#[cfg(not(target_arch = "wasm32"))]
pub type PinnedStream = Pin<Box<dyn Stream<Item = crate::Result<Bytes>> + Send + Sync>>;
#[cfg(target_arch = "wasm32")]
pub type PinnedStream = Pin<Box<dyn Stream<Item = crate::Result<Bytes>>>>;

#[cfg(not(target_arch = "wasm32"))]
pub type BoxedDeserializer<'a, T> = Box<
    dyn FnOnce(PinnedStream) -> Pin<Box<dyn Future<Output = crate::Result<T>> + Send + Sync + 'a>>
        + Send
        + Sync
        + 'a,
>;
#[cfg(target_arch = "wasm32")]
pub type BoxedDeserializer<'a, T> =
    Box<dyn FnOnce(PinnedStream) -> Pin<Box<dyn Future<Output = crate::Result<T>> + 'a>> + 'a>;

async fn collect_stream(mut stream: PinnedStream) -> crate::Result<Bytes> {
    let mut final_result = Vec::new();

    while let Some(res) = stream.next().await {
        final_result.extend(&res?);
    }

    Ok(final_result.into())
}

/// An HTTP response.
///
/// The type parameter `T` is a marker type that indicates what the caller should expect to be able to deserialize the body into.
/// Service client methods should return a `Response<SomeModel>` where `SomeModel` is the service-specific response type.
/// For example, a service client method that returns a list of secrets should return `Response<ListSecretsResponse>`.
///
/// Given a `Response<T>`, a user can deserialize the body into the intended body type `T` by calling [`Response::deserialize_body`].
/// However, because the type `T` is just a marker type, the user can also deserialize the body into a different type by calling [`Response::deserialize_body_into`].
pub struct Response<T = Bytes> {
    status: StatusCode,
    headers: Headers,
    body: ResponseBody<T>,
}

impl Response<Bytes> {
    /// Create an HTTP response from an asynchronous stream of bytes.
    pub fn from_stream(status: StatusCode, headers: Headers, stream: PinnedStream) -> Self {
        Self {
            status,
            headers,
            body: ResponseBody::new(stream),
        }
    }

    /// Create an HTTP response from raw bytes.
    pub fn from_bytes(status: StatusCode, headers: Headers, bytes: impl Into<Bytes>) -> Self {
        Self {
            status,
            headers,
            body: ResponseBody::from_bytes(bytes),
        }
    }
}

impl<T> Response<T> {
    pub fn new(status: StatusCode, headers: Headers, body: ResponseBody<T>) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    /// Get the status code from the response.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the headers from the response.
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Deconstruct the HTTP response into its components.
    pub fn deconstruct(self) -> (StatusCode, Headers, ResponseBody<T>) {
        (self.status, self.headers, self.body)
    }

    /// Fetches the entire body and returns it as raw bytes.
    ///
    /// This method will force the entire body to be downloaded from the server and consume the response.
    /// If you want to parse the body into a type, use [`read_body`](Response::deserialize_body) instead.
    pub fn into_body(self) -> ResponseBody<T> {
        self.body
    }

    #[cfg(feature = "json")]
    pub fn with_json_body<U>(self) -> Response<U>
    where
        U: DeserializeOwned,
    {
        let (status, headers, body) = self.deconstruct();
        Response::new(status, headers, body.to_json())
    }

    #[cfg(feature = "xml")]
    pub fn with_xml_body<U>(self) -> Response<U>
    where
        U: DeserializeOwned,
    {
        let (status, headers, body) = self.deconstruct();
        Response::new(status, headers, body.to_xml())
    }
}

impl<'a, T> fmt::Debug for Response<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            // TODO: Sanitize headers and emit body as "(body)".
            .finish_non_exhaustive()
    }
}

/// A response body stream.
///
/// This body can either be streamed or collected into [`Bytes`].
#[pin_project::pin_project]
pub struct ResponseBody<T = Bytes> {
    #[pin]
    stream: PinnedStream,
    deserializer: BoxedDeserializer<'static, T>,
}

impl ResponseBody<Bytes> {
    /// Create a new [`ResponseBody`] from an async stream of bytes.
    fn new(stream: PinnedStream) -> Self {
        Self {
            stream,
            deserializer: Box::new(|stream| Box::pin(collect_stream(stream))),
        }
    }

    /// Create a new [`ResponseBody`] from a byte slice.
    fn from_bytes(bytes: impl Into<Bytes>) -> Self {
        let bytes = bytes.into();
        Self::new(Box::pin(futures::stream::once(async move { Ok(bytes) })))
    }
}

impl<T: 'static> IntoFuture for ResponseBody<T> {
    type Output = crate::Result<T>;

    #[cfg(not(target_arch = "wasm32"))]
    type IntoFuture = Pin<Box<dyn Future<Output = crate::Result<T>> + Send + Sync + 'static>>;

    #[cfg(target_arch = "wasm32")]
    type IntoFuture = Pin<Box<dyn Future<Output = crate::Result<T>> + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async { (self.deserializer)(self.stream).await })
    }
}

impl<T> ResponseBody<T> {
    /// Collect the stream into a [`Bytes`] collection.
    pub async fn collect_bytes(self) -> crate::Result<Bytes> {
        collect_stream(self.stream).await
    }

    /// Collect the stream into a [`String`].
    pub async fn collect_string(self) -> crate::Result<String> {
        std::str::from_utf8(&self.collect_bytes().await?)
            .context(
                ErrorKind::DataConversion,
                "response body was not utf-8 like expected",
            )
            .map(ToOwned::to_owned)
    }

    /// Deserialize the JSON stream into type `T`.
    #[cfg(feature = "json")]
    pub fn to_json<U>(self) -> ResponseBody<U>
    where
        U: DeserializeOwned,
    {
        ResponseBody {
            stream: self.stream,
            deserializer: Box::new(|stream| {
                Box::pin(async {
                    let bytes = collect_stream(stream).await?;
                    crate::json::from_json(bytes)
                })
            }),
        }
    }

    /// Deserialize the XML stream into type `T`.
    #[cfg(feature = "xml")]
    pub fn to_xml<U>(self) -> ResponseBody<U>
    where
        U: DeserializeOwned,
    {
        ResponseBody {
            stream: self.stream,
            deserializer: Box::new(|stream| {
                Box::pin(async {
                    let bytes = collect_stream(stream).await?;
                    crate::xml::read_xml(&bytes)
                })
            }),
        }
    }
}

impl<'a, T> Stream for ResponseBody<T> {
    type Item = crate::Result<Bytes>;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        this.stream.poll_next(cx)
    }
}

impl<'a, T> fmt::Debug for ResponseBody<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ResponseBody")
    }
}

#[cfg(test)]
mod tests {
    mod json {
        use crate::http::headers::Headers;
        use crate::http::Response;
        use http_types::StatusCode;
        use serde::Deserialize;

        /// An example JSON-serialized response type.
        #[derive(Deserialize)]
        struct GetSecretResponse {
            name: String,
            value: String,
        }

        /// An example JSON-serialized list response type.
        #[derive(Deserialize)]
        struct GetSecretListResponse {
            value: Vec<GetSecretResponse>,
            #[serde(rename = "nextLink")]
            next_link: Option<String>,
        }

        /// A sample service client function.
        fn get_secret() -> Response<GetSecretResponse> {
            Response::from_bytes(
                StatusCode::Ok,
                Headers::new(),
                r#"{"name":"my_secret","value":"my_value"}"#,
            )
            .with_json_body()
        }

        /// A sample service client function to return a list of secrets.
        fn list_secrets() -> Response<GetSecretListResponse> {
            Response::from_bytes(
                StatusCode::Ok,
                Headers::new(),
                r#"{"value":[{"name":"my_secret","value":"my_value"}],"nextLink":"?page=2"}"#,
            )
            .with_json_body()
        }

        #[tokio::test]
        pub async fn deserialize_default_type() -> Result<(), Box<dyn std::error::Error>> {
            let response = get_secret();
            let secret = response.into_body().await?;
            assert_eq!(secret.name, "my_secret");
            assert_eq!(secret.value, "my_value");
            Ok(())
        }

        #[tokio::test]
        pub async fn deserialize_alternate_type() -> Result<(), Box<dyn std::error::Error>> {
            #[derive(Deserialize)]
            struct MySecretResponse {
                #[serde(rename = "name")]
                yon_name: String,
                #[serde(rename = "value")]
                yon_value: String,
            }

            let response = get_secret();
            let secret: MySecretResponse = response.with_json_body().into_body().await?;
            assert_eq!(secret.yon_name, "my_secret");
            assert_eq!(secret.yon_value, "my_value");
            Ok(())
        }

        #[tokio::test]
        async fn deserialize_pageable_from_body() {
            // We need to efficiently deserialize the body twice to get the "nextLink" but return it to the caller.
            let response = list_secrets();
            let (status, headers, body) = response.deconstruct();
            let bytes = body.collect_bytes().await.expect("collect response");
            let model: GetSecretListResponse =
                crate::json::from_json(bytes.clone()).expect("deserialize GetSecretListResponse");
            assert_eq!(status, StatusCode::Ok);
            assert_eq!(model.value.len(), 1);
            assert_eq!(model.next_link, Some("?page=2".to_string()));

            let response: Response<GetSecretListResponse> =
                Response::from_bytes(status, headers, bytes).with_json_body();
            assert_eq!(response.status(), StatusCode::Ok);
            let model = response
                .into_body()
                .await
                .expect("deserialize GetSecretListResponse again");
            assert_eq!(model.next_link, Some("?page=2".to_string()));
        }
    }

    #[cfg(feature = "xml")]
    mod xml {
        use crate::http::headers::Headers;
        use crate::http::Response;
        use http_types::StatusCode;
        use serde::Deserialize;

        /// An example XML-serialized response type.
        #[derive(Deserialize)]
        struct GetSecretResponse {
            name: String,
            value: String,
        }

        /// A sample service client function.
        fn get_secret() -> Response<GetSecretResponse> {
            Response::from_bytes(
                StatusCode::Ok,
                Headers::new(),
                "<GetSecretResponse><name>my_secret</name><value>my_value</value></GetSecretResponse>",
            ).with_xml_body()
        }

        #[tokio::test]
        pub async fn deserialize_default_type() -> Result<(), Box<dyn std::error::Error>> {
            let response = get_secret();
            let secret = response.into_body().await?;
            assert_eq!(secret.name, "my_secret");
            assert_eq!(secret.value, "my_value");
            Ok(())
        }

        #[tokio::test]
        pub async fn deserialize_alternate_type() -> Result<(), Box<dyn std::error::Error>> {
            #[derive(Deserialize)]
            struct MySecretResponse {
                #[serde(rename = "name")]
                yon_name: String,
                #[serde(rename = "value")]
                yon_value: String,
            }

            let response = get_secret();
            let secret: MySecretResponse = response.with_xml_body().into_body().await?;
            assert_eq!(secret.yon_name, "my_secret");
            assert_eq!(secret.yon_value, "my_value");
            Ok(())
        }
    }
}
