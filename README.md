# Azure SDK for Rust

This repository is for the development of the [unofficial](https://github.com/Azure/azure-sdk-for-rust/blob/main/FAQ.md#why-is-it-unofficial) Azure SDK for Rust.

## Crates

[All Azure SDK for Rust crates](https://crates.io/teams/github:azure:azure-sdk-publish-rust) are published on crates.io.

### SDK
These [SDK crates](sdk) are available:
- [azure_core](https://crates.io/crates/azure_core)
- [azure_identity](https://crates.io/crates/azure_identity)
- [azure_data_cosmos](https://crates.io/crates/azure_data_cosmos)
- [azure_data_tables](https://crates.io/crates/azure_data_tables)
- [azure_iot_hub](https://crates.io/crates/azure_iot_hub)
- [azure_security_keyvault](https://crates.io/crates/azure_security_keyvault)
- [azure_storage_blobs](https://crates.io/crates/azure_storage_blobs)
- [azure_storage_datalake](https://crates.io/crates/azure_storage_datalake)
- [azure_storage_queues](https://crates.io/crates/azure_storage_queues)

### Services
Azure service crates generated from [Azure REST API Specifications](https://github.com/Azure/azure-rest-api-specs) are available in [services](services).

## Status

🚨 WARNING 🚨: This project is under active development. Be aware that large breaking changes will happen before 1.0 is reached.

This project is the successor to the `azure_sdk*` crates from [MindFlavor/AzureSDKForRust](https://github.com/MindFlavor/AzureSDKForRust). The crates have been renamed, so those older crates should be considered fully deprecated. See [history](HISTORY.md) for more details.

## Project Structure

Each supported Azure service is its own separate crate.

Building each crate should be as straight forward as `cargo build`, but check each crate's README for more specific information.

### Mock testing framework

This library comes with a testing framework that executes against prerecorded sessions to quickly validate code changes without incurring in Azure costs. You can read more about it in the [Mock testing framework's README](https://github.com/Azure/azure-sdk-for-rust/tree/feature/track2/doc/mock_transport.md).

## Contributing

This project welcomes contributions and suggestions.  Most contributions require you to agree to a
Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us
the rights to use your contribution. For details, visit https://cla.opensource.microsoft.com.

When you submit a pull request, a CLA bot will automatically determine whether you need to provide
a CLA and decorate the PR appropriately (e.g., status check, comment). Simply follow the instructions
provided by the bot. You will only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

### Development Environment

This repository supports the [devenv](https://devenv.sh) tool (not to be confused with the name of Visual Studio's executable on Windows!) which uses the Nix Package Manager to create reproducible development environments on Linux machines.
We recommend using Linux (including Windows Subsystem for Linux) and devenv to develop this repo, but neither are mandatory.

To use our pre-configured development environment, first configure the devenv tool itself by following the guide in [the devenv documentation](https://devenv.sh/getting-started/).
Then, navigate to this repo and run `devenv shell` to enter a shell with all the required tools installed and configured:

```bash
$ devenv shell
• Building shell ...
• Using Cachix: devenv
• Trusting devenv.cachix.org on first use with the public key devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=
✔ Building shell in 27.3s.
• Entering shell
You have entered the Development Environment for the Azure SDK for Rust
```

> [!IMPORTANT]
> Running `devenv shell` will install packages necessary to develop the Azure SDK for Rust in to your Nix Store (`/nix/store`)

Any required package will be installed using the [Nix Package Manager](https://nixos.org/).
This means that packages installed by our development environment will not interfere with your normal machine configuration.
When you leave the shell with `exit`, your shell will be returned to it's original state.

You'll know you're in the Azure SDK for Rust development environment by checking the `AZRUST_SHELL` environment variable.
You can use this variable, and/or various other shell prompt plugins, to mark your prompt to indicate you're within the Azure SDK for Rust development environment.

```bash
$ echo $AZRUST_SHELL
1
```

In addition, if you install the [direnv](https://direnv.net/docs/installation.html#from-system-packages) tool, it will automatically activate the development environment any time your shell's current directory is within this repo, and deactivate it when you leave the repo.

```bash
~ $ cd azure-sdk-for-rust
direnv: loading ~/code/Azure/azure-sdk-for-rust/.envrc
direnv: loading https://raw.githubusercontent.com/cachix/devenv/95f329d49a8a5289d31e0982652f7058a189bfca/direnvrc (sha256-d+8cBpDfDBj41inrADaJt+bDWhOktwslgoP5YiGJ1v0=)
direnv: using devenv
direnv: using cached devenv shell
You have entered the Development Environment for the Azure SDK for Rust
direnv: export +AR +AS +AZRUST_SHELL +CC +CONFIG_SHELL +CXX +DEVENV_DOTFILE +DEVENV_PROFILE +DEVENV_ROOT +DEVENV_RUNTIME +DEVENV_STATE +IN_NIX_SHELL +LD +NIX_BINTOOLS +NIX_BINTOOLS_WRAPPER_TARGET_HOST_x86_64_unknown_linux_gnu +NIX_CC +NIX_CC_WRAPPER_TARGET_HOST_x86_64_unknown_linux_gnu +NIX_CFLAGS_COMPILE +NIX_ENFORCE_NO_NATIVE +NIX_HARDENING_ENABLE +NIX_LDFLAGS +NIX_PKG_CONFIG_WRAPPER_TARGET_HOST_x86_64_unknown_linux_gnu +NIX_STORE +NM +OBJCOPY +OBJDUMP +PKG_CONFIG +PKG_CONFIG_PATH +RANLIB +READELF +SIZE +SOURCE_DATE_EPOCH +STRINGS +STRIP +cmakeFlags +configureFlags +mesonFlags +name +system ~PATH ~XDG_DATA_DIRS
~/azure-sdk-for-rust $ echo $AZRUST_SHELL
1
~/azure-sdk-for-rust $ cd ..
direnv: unloading
~ $ echo $AZRUST_SHELL

~ $
```