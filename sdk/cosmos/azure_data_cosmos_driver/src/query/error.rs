use std::{borrow::Cow, fmt::Display};

#[derive(Debug)]
pub enum Error {
    /// An error occurred due to an invalid query plan.
    ///
    /// The associated string is an error message describing the error.
    /// This error is not recoverable by the user.
    QueryPlanInvalid(Cow<'static, str>),

    /// The partition key range requested was not found. This is a fatal error and the user should not retry.
    PartitionNotFound(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::QueryPlanInvalid(message) => write!(f, "query plan is invalid: {}", message),
            Error::PartitionNotFound(id) => write!(f, "partition range not found: '{}'", id),
        }
    }
}
