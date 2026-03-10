# Pre-Push Validation Results

**Branch:** `test/rerun-s3`
**Changed files:**

- `eng/scripts/Analyze-Code.ps1` — skip reinstalling cargo-audit when the required version is already present
- `eng/scripts/Invoke-PrePush.ps1` — new pre-push verification script
- `sdk/core/azure_core/README.md` — added a documentation link

## Summary

| Step | Status | Log file |
|------|--------|----------|
| Analysis | **FAILED** | `analysis.log` |
| Tests | **FAILED** | `tests.log` |
| Spell check | PASSED | `spellcheck.log` |
| Link check | **FAILED** | `linkcheck.log` |

**Overall result: FAILED** (3 failed, 1 passed, 0 skipped)

## Failure Details

### 1. Analysis — `cargo audit` hangs on network fetch

**What went wrong:**
The analysis step runs `cargo audit` which attempts to fetch the RustSec advisory
database from `https://github.com/RustSec/advisory-db.git`. In this environment
the SSH agent rejects all signing keys, and the fetch hangs indefinitely waiting
for authentication or a network timeout:

```
sign_and_send_pubkey: signing failed for ED25519 "GitHub Authentication" from agent: agent refused operation
sign_and_send_pubkey: signing failed for ED25519 "Microsoft EMU Key" from agent: agent refused operation
```

Because the script never gets past `cargo audit`, subsequent analysis checks
(`cargo check`, `cargo fmt --check`, `cargo clippy`, `cargo doc`,
`verify-dependencies`, `verify-keywords`, `check_api_superset`) never execute.

Manual verification of the remaining sub-steps:

- `cargo fmt --all -- --check` — **passes**
- `cargo check` / `cargo clippy` / `cargo doc` — **fail** due to missing
  `libssl-dev` (environment issue, not a code defect)

**Proposed fix:**
Configure git to prefer HTTPS without SSH fallback, or pre-seed the advisory
database cache so `cargo audit` does not require a network fetch:

```bash
# Option A: ensure HTTPS-only git access
git config --global url."https://github.com/".insteadOf "git@github.com:"

# Option B: pre-fetch the advisory DB when online
cargo audit fetch

# Option C: skip the audit step locally
pwsh -File eng/scripts/Invoke-PrePush.ps1 -SkipAnalysis
```

For the `libssl-dev` build failure, install the development package:

```bash
sudo apt-get install libssl-dev   # or equivalent for your distro
```

---

### 2. Tests — `azure_core_amqp` test setup fails on stale clone

**What went wrong:**
`Test-Packages.ps1` runs each package's `Test-Setup.ps1` before testing.
The `azure_core_amqp` setup script tries to clone `azure-amqp` into
`../TestArtifacts/`, but a directory from a previous run already exists:

```
> git clone https://github.com/Azure/azure-amqp.git --revision d82a86455c3459c5628bc95b25511f6e8a065598
fatal: destination path 'azure-amqp' already exists and is not an empty directory.
Command failed to execute
```

The test script exits immediately on this failure (`exit 1` in
`Test-Packages.ps1` line 82), so **no tests for any package actually run**.

**Proposed fix:**
Remove the stale clone directory before re-running:

```bash
rm -rf ../TestArtifacts/azure-amqp
```

Alternatively, the `Test-Setup.ps1` script could be made idempotent by checking
whether the directory already exists and is at the expected revision before
attempting a fresh clone.

---

### 3. Link check — broken URL in `sdk/core/azure_core/README.md`

**What went wrong:**
The commit `559842bd4` ("test: rerun s3 - broken link") added a link to a
non-existent Microsoft Learn page:

```markdown
See also our [nonexistent documentation page](https://learn.microsoft.com/this-page-does-not-exist-12345) for more details.
```

The `Verify-Links.ps1` link checker reports this as a 404:

```
[404] broken link https://learn.microsoft.com/this-page-does-not-exist-12345
Checked 27 links with 1 broken link(s) found.
```

**Note:** The link check step also hung when run inside the `Invoke-PrePush.ps1`
pipeline (likely due to the `Tee-Object` output buffering with slow HTTP
requests), but the failure was confirmed by running `Verify-Links.ps1` directly.

**Proposed fix:**
Remove the broken link or replace it with a valid URL:

```diff
- See also our [nonexistent documentation page](https://learn.microsoft.com/this-page-does-not-exist-12345) for more details.
+ See also our [Rust SDK documentation](https://learn.microsoft.com/azure/developer/rust) for more details.
```

Or, if this line was added purely for testing, simply delete it.

## Steps That Passed

### Spell check

All 3 changed files were checked by cSpell. No spelling errors were found.
