<div class="warning">

Replacing the HTTP client factory or the async runtime puts the SDK outside the configuration that Microsoft validates and ships. As a result, Microsoft cannot provide 24/7 support for the SDK through Azure Support for operations that run with a non-default plug point. When a support ticket is opened, the engineer will ask you to reproduce the issue with the default reqwest HTTP client / tokio async runtime before investigation can proceed. See the [Azure Support policy](https://azure.microsoft.com/support/legal/) for full details.

The `DiagnosticsContext` (see the `azure_data_cosmos_driver::diagnostics` module) exposed via every Cosmos response carries `custom_http_client` / `custom_async_runtime` flags that record which plug points were in use for that operation.

</div>
