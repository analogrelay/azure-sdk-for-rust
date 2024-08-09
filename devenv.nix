# This file is a config file for the 'devenv' tool (https://devenv.sh, NOT to be confused with the executable name for Visual Studio: devenv.exe)
# To use this, install 'devenv' (https://devenv.sh/getting-started) and run 'devenv shell' in the root of the repo.
# Dependencies necessary to build this repo will be automatically installed and activated in your shell.
{ pkgs, lib, config, inputs, ... }:

{
    env = {
        AZRUST_SHELL = 1;
    };

    packages = [
        pkgs.rustup
        pkgs.pkg-config
        pkgs.openssl
    ];

    enterShell = ''
        echo "You have entered the Development Environment for the Azure SDK for Rust"
    '';
}