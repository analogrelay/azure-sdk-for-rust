---
name: validate-changes
description: Validate code changes by running formatting, linting, tests, and spell checking on changed crates.
---

# Validate Changes

Run CI-equivalent validation on changes in the current branch, scoped to only the crates and files that changed.

## Usage

Run from the repository root:

```bash
pwsh .github/skills/validate-changes/Validate-Changes.ps1 -Fix
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `-BaseBranch` | string | Git ref for the base branch. Auto-detected if omitted. |
| `-IncludeEmulatorTests` | switch | Run Cosmos emulator-dependent tests. |
| `-SkipTests` | switch | Skip the `cargo test` step entirely. |
| `-Fix` | switch | Auto-apply formatting via `cargo fmt`. Without this, formatting is only checked. |

### Examples

```bash
# Validate with auto-fix formatting
pwsh .github/skills/validate-changes/Validate-Changes.ps1 -Fix

# Validate against a specific base branch
pwsh .github/skills/validate-changes/Validate-Changes.ps1 -Fix -BaseBranch origin/release/azure_data_cosmos-previews

# Include emulator tests
pwsh .github/skills/validate-changes/Validate-Changes.ps1 -Fix -IncludeEmulatorTests

# Skip tests (faster validation for non-code changes)
pwsh .github/skills/validate-changes/Validate-Changes.ps1 -Fix -SkipTests
```

## What it validates

The script detects changed crates by comparing the current branch to the base branch, then runs:

1. **`cargo fmt`** — Format check (or auto-fix with `-Fix`)
2. **`cargo clippy`** — Lint check per changed crate
3. **`cargo doc`** — Documentation build per changed crate (catches broken intra-doc links)
4. **`cargo test`** — Tests per changed crate (skippable with `-SkipTests`)
5. **Spell check** — cSpell on changed files only, via `eng/common/scripts/check-spelling-in-changed-files.ps1`
6. **Markdown lint** — markdownlint on changed `.md` files, via the `lint-markdown` skill

The script outputs a pass/fail summary for each step.

## Agent instructions

When invoking this skill after completing a task:

1. **Run all steps to completion.** Collect issues from every step into a single report. Only skip a step if a prior step produced a blocking error (e.g., a syntax error prevents compilation, which blocks clippy, doc, and test steps).

2. **`cargo fmt` is the only auto-fix.** Always pass `-Fix` so formatting is applied automatically. Formatting is deterministic and non-controversial.

3. **Prompt before emulator tests.** Before adding `-IncludeEmulatorTests`, ask the user whether the Cosmos emulator is running and whether emulator tests should be included.

4. **Present a single consolidated report.** After the script completes, describe all issues found across all steps to the user, grouped by step. Propose fixes for each issue. Wait for user confirmation before applying any changes (other than formatting, which was already applied by `-Fix`).

5. **Fixing spelling errors.** For spelling issues, either correct typos in source code or add legitimate terms to dictionary files:
   - **Crate names**: `eng/dict/crates.txt`
   - **Service-specific terms**: `sdk/<service>/.dict.txt` (e.g., `sdk/cosmos/.dict.txt`)
   - **Global dictionary**: `.vscode/cspell.json`

6. **Fixing markdown lint issues.** Re-run markdownlint with `--fix` to auto-fix supported issues, or apply fixes manually as described by the linter output.
