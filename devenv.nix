# This file is a config file for the 'devenv' tool (https://devenv.sh, NOT to be confused with the executable name for Visual Studio: devenv.exe)
# To use this, install 'devenv' (https://devenv.sh/getting-started) and run 'devenv shell' in the root of the repo.
# Dependencies necessary to build this repo will be automatically installed and activated in your shell.
{ pkgs, lib, config, inputs, ... }:

{
    packages = [
        pkgs.rustup
        pkgs.pkg-config
        pkgs.openssl
    ];
}