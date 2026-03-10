# Pre-Push Validation Results

**Command:** `pwsh -NoProfile -File eng/scripts/Invoke-PrePush.ps1 -OutputDir ./target/prepush`
**Branch:** `test/rerun-s1` (target: `main`)
**Overall result:** **FAILED**

## Summary

| Step | Status | Log file |
|------|--------|----------|
| analysis | **FAILED** | `analysis.log` |
| tests | **FAILED** | `tests.log` |
| spellcheck | PASSED | `spellcheck.log` |
| linkcheck | SKIPPED | *(no changed markdown files)* |

**1 passed, 2 failed, 1 skipped**

---

## Failure 1: Analysis (`analysis.log`)

### What happened

The `cargo check --package azure_core --all-features --all-targets --keep-going` command failed because the OpenSSL development libraries are not installed on this machine. The `openssl-sys` build script could not locate the OpenSSL installation directory.

Because `Analyze-Code.ps1` exits on the first `Invoke-LoggedCommand` failure, the subsequent steps — `cargo fmt --check`, `cargo clippy`, and `cargo doc` — were never executed.

Running `cargo fmt --check` separately reveals a **formatting violation** in the changed file `sdk/core/typespec_client_core/src/base64.rs` (line 11):

```
-        general_purpose::{   GeneralPurpose,    GeneralPurposeConfig   },
+        general_purpose::{GeneralPurpose, GeneralPurposeConfig},
```

Extra whitespace was introduced inside the braces of the `use` import.

### Proposed fix

1. **Formatting issue (the actual code problem in this commit):** Run `cargo fmt -p typespec_client_core` to fix the spacing in `base64.rs`, or manually revert line 11 to:

   ```rust
   general_purpose::{GeneralPurpose, GeneralPurposeConfig},
   ```

2. **OpenSSL environment issue (local environment, not a code bug):** Install the OpenSSL development package so that `cargo check --all-features` can compile `openssl-sys`. On Ubuntu/Debian:

   ```bash
   sudo apt-get install libssl-dev pkg-config
   ```

   Alternatively, set `OPENSSL_DIR` to point at an existing OpenSSL installation.

---

## Failure 2: Tests (`tests.log`)

### What happened

`Test-Packages.ps1` iterates over all detected changed packages and runs test setup scripts before testing each one. The very first package alphabetically is `azure_core_amqp`, whose `Test-Setup.ps1` attempts to clone `https://github.com/Azure/azure-amqp.git` into a working directory. The clone failed because the destination directory `azure-amqp` already exists from a previous test run:

```
fatal: destination path 'azure-amqp' already exists and is not an empty directory.
Command failed to execute: git clone https://github.com/Azure/azure-amqp.git ...
```

Because `Test-Packages.ps1` calls `Invoke-LoggedCommand` for the setup script **without** `-DoNotExitOnFailedExitCode`, the script exits immediately on this first setup failure. No tests are actually executed for **any** package (including `typespec_client_core`, the one that actually changed).

### Proposed fix

1. **Clear stale test artifacts (immediate workaround):** Remove the leftover clone directory before re-running:

   ```bash
   rm -rf ../TestArtifacts/azure-amqp
   ```

2. **Scope the test run to the changed package only (recommended):** Since only `typespec_client_core` changed, pass `-PackageNames typespec_client_core` to avoid running test setup for unrelated packages:

   ```bash
   pwsh -NoProfile -File eng/scripts/Invoke-PrePush.ps1 \
     -OutputDir ./target/prepush \
     -PackageNames typespec_client_core
   ```

   Note: This still requires fixing the formatting issue above so the analysis step can pass `cargo fmt --check`.
