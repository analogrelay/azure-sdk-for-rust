// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Canonical doc-section text for plug-point support-policy notices.
//!
//! The `support_policy_notice!()` macro expands to a single string literal so
//! it can be embedded directly in `#[doc = ...]` attributes on every public
//! API that lets a caller replace the default HTTP client factory or async
//! runtime. Keep the wording in exactly one place so the documentation
//! across the driver and SDK stays in lock-step.

/// Returns the canonical "Azure Support" doc-section wording as a `&'static str`
/// literal. Intended for use with `#[doc = support_policy_notice!()]` on public
/// setters and re-exports that accept a custom HTTP client factory or async
/// runtime.
#[macro_export]
#[doc(hidden)]
macro_rules! support_policy_notice {
    () => {
        "\n\n# Azure Support\n\n\
Replacing the HTTP client factory or the async runtime puts the SDK outside the configuration that Microsoft validates and ships. \
As a result, Microsoft cannot provide 24/7 support for the SDK through Azure Support for operations that run with a non-default plug point. \
When a support ticket is opened, the engineer will ask you to reproduce the issue with the default reqwest HTTP client / tokio async runtime before investigation can proceed. \
See the [Azure Support policy](https://azure.microsoft.com/support/legal/) for full details.\n\n\
The [`DiagnosticsContext`](crate::diagnostics::DiagnosticsContext) exposed via every Cosmos response carries `custom_http_client` / `custom_async_runtime` flags that record which plug points were in use for that operation.\n"
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn notice_mentions_key_phrases() {
        let notice: &str = crate::support_policy_notice!();
        assert!(notice.contains("# Azure Support"));
        assert!(notice.contains("24/7 support"));
        assert!(notice.contains("custom_http_client"));
        assert!(notice.contains("custom_async_runtime"));
    }
}
