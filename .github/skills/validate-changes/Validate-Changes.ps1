#!/usr/bin/env pwsh

#Requires -Version 7.0

<#
.SYNOPSIS
Validates changes on the current branch against the base branch.

.DESCRIPTION
Detects changed crates by comparing the current branch to its base branch,
then runs formatting, linting, documentation, testing, spell checking, and
markdown linting. Outputs a summary of pass/fail per step.

This script mirrors key CI checks from eng/scripts/Analyze-Code.ps1 and
eng/scripts/Test-Packages.ps1, scoped to only the crates that changed.

.PARAMETER BaseBranch
Git ref for the base branch. If not provided, the script attempts to
auto-detect using the upstream tracking branch or falls back to origin/main.

.PARAMETER IncludeEmulatorTests
Run Cosmos emulator-dependent tests in addition to standard tests.

.PARAMETER SkipTests
Skip the cargo test step entirely.

.PARAMETER Fix
Auto-apply formatting fixes via cargo fmt. Without this flag, formatting
is checked but not modified.

.EXAMPLE
pwsh .github/skills/validate-changes/Validate-Changes.ps1 -Fix

.EXAMPLE
pwsh .github/skills/validate-changes/Validate-Changes.ps1 -BaseBranch origin/release/azure_data_cosmos-previews -IncludeEmulatorTests
#>

param(
  [string]$BaseBranch,
  [switch]$IncludeEmulatorTests,
  [switch]$SkipTests,
  [switch]$Fix
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version 2.0

# Import common helpers for logging and command invocation
. ([System.IO.Path]::Combine($PSScriptRoot, '..', '..', '..', 'eng', 'common', 'scripts', 'common.ps1'))

$RepoRoot = Resolve-Path ([System.IO.Path]::Combine($PSScriptRoot, '..', '..', '..'))

# Track results per step
$stepResults = [ordered]@{}

function Invoke-Step {
  param(
    [string]$Name,
    [scriptblock]$Action
  )

  Write-Host "`n========================================" -ForegroundColor Cyan
  Write-Host " Step: $Name" -ForegroundColor Cyan
  Write-Host "========================================`n" -ForegroundColor Cyan

  $global:LASTEXITCODE = 0

  try {
    $result = & $Action
    if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) {
      $stepResults[$Name] = 'FAILED'
    }
    else {
      $stepResults[$Name] = 'PASSED'
    }
  }
  catch {
    Write-Host "Step '$Name' encountered an error: $_" -ForegroundColor Red
    $stepResults[$Name] = 'FAILED'
  }
}

# ---------------------------------------------------------------------------
# Step 0: Detect base branch and changed packages
# ---------------------------------------------------------------------------

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host " Step 0: Detect base branch and changed packages" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

if (-not $BaseBranch) {
  # Try upstream tracking branch
  $tracking = git rev-parse --abbrev-ref '@{upstream}' 2>$null
  if ($LASTEXITCODE -eq 0 -and $tracking) {
    $BaseBranch = $tracking
    Write-Host "Auto-detected base branch from tracking: $BaseBranch"
  }
  else {
    $BaseBranch = 'origin/main'
    Write-Host "Could not detect tracking branch, falling back to: $BaseBranch"
  }
}
else {
  Write-Host "Using provided base branch: $BaseBranch"
}

$mergeBase = git merge-base HEAD $BaseBranch 2>$null
if ($LASTEXITCODE -ne 0 -or -not $mergeBase) {
  LogError "Could not determine merge base between HEAD and $BaseBranch"
  exit 1
}
Write-Host "Merge base: $mergeBase"

$changedFiles = git diff --name-only "$mergeBase...HEAD" 2>$null
if (-not $changedFiles) {
  Write-Host "No changed files detected. Nothing to validate."
  exit 0
}

$changedFileList = $changedFiles -split "`n" | Where-Object { $_ -ne '' }
Write-Host "Changed files ($($changedFileList.Count)):"
foreach ($f in $changedFileList) {
  Write-Host "  $f"
}

# Derive changed crate names by finding Cargo.toml files in ancestor directories
$changedCrates = @{}
$cargoMetadata = cargo metadata --format-version 1 --no-deps 2>$null | ConvertFrom-Json -AsHashtable
if (-not $cargoMetadata) {
  LogError "Failed to run cargo metadata"
  exit 1
}

$packagesByDir = @{}
foreach ($pkg in $cargoMetadata.packages) {
  $dir = (Split-Path $pkg.manifest_path -Parent) -replace '\\', '/'
  # Normalize to relative path from repo root
  $repoRootNormalized = "$RepoRoot" -replace '\\', '/'
  if ($dir.StartsWith($repoRootNormalized)) {
    $dir = $dir.Substring($repoRootNormalized.Length).TrimStart('/')
  }
  $packagesByDir[$dir] = $pkg.name
}

foreach ($file in $changedFileList) {
  $filePath = $file -replace '\\', '/'
  foreach ($dir in $packagesByDir.Keys) {
    if ($filePath.StartsWith("$dir/") -or $filePath -eq $dir) {
      $changedCrates[$packagesByDir[$dir]] = $true
    }
  }
}

$crateNames = @($changedCrates.Keys | Sort-Object)

if ($crateNames.Count -eq 0) {
  Write-Host "`nNo crate changes detected (changes may be in non-crate files)."
  Write-Host "Skipping cargo validation steps.`n"
}
else {
  Write-Host "`nChanged crates ($($crateNames.Count)):"
  foreach ($c in $crateNames) {
    Write-Host "  $c"
  }
}

$changedMarkdown = $changedFileList | Where-Object { $_ -match '\.md$' }

# ---------------------------------------------------------------------------
# Step 1: cargo fmt
# ---------------------------------------------------------------------------

if ($crateNames.Count -gt 0) {
  Invoke-Step "cargo fmt" {
    $fmtFailed = $false
    foreach ($crate in $crateNames) {
      if ($Fix) {
        Write-Host "Formatting $crate..."
        Invoke-LoggedCommand "cargo fmt -p $crate" -GroupOutput -DoNotExitOnFailedExitCode
        if ($LASTEXITCODE -ne 0) { $fmtFailed = $true }
      }
      else {
        Write-Host "Checking format for $crate..."
        Invoke-LoggedCommand "cargo fmt -p $crate -- --check" -GroupOutput -DoNotExitOnFailedExitCode
        if ($LASTEXITCODE -ne 0) {
          LogWarning "Formatting issues found in $crate. Run with -Fix to auto-apply."
          $fmtFailed = $true
        }
      }
    }
    if ($fmtFailed) { $global:LASTEXITCODE = 1 } else { $global:LASTEXITCODE = 0 }
  }

  # ---------------------------------------------------------------------------
  # Step 2: cargo clippy
  # ---------------------------------------------------------------------------

  Invoke-Step "cargo clippy" {
    $clippyFailed = $false
    foreach ($crate in $crateNames) {
      Write-Host "Running clippy on $crate..."
      Invoke-LoggedCommand "cargo clippy -p $crate --all-features --all-targets --no-deps" -GroupOutput -DoNotExitOnFailedExitCode
      if ($LASTEXITCODE -ne 0) { $clippyFailed = $true }
    }
    if ($clippyFailed) { $global:LASTEXITCODE = 1 } else { $global:LASTEXITCODE = 0 }
  }

  # ---------------------------------------------------------------------------
  # Step 3: cargo doc
  # ---------------------------------------------------------------------------

  Invoke-Step "cargo doc" {
    $docFailed = $false
    foreach ($crate in $crateNames) {
      Write-Host "Building docs for $crate..."
      Invoke-LoggedCommand "cargo doc -p $crate --no-deps --all-features" -GroupOutput -DoNotExitOnFailedExitCode
      if ($LASTEXITCODE -ne 0) { $docFailed = $true }
    }
    if ($docFailed) { $global:LASTEXITCODE = 1 } else { $global:LASTEXITCODE = 0 }
  }

  # ---------------------------------------------------------------------------
  # Step 4: cargo test
  # ---------------------------------------------------------------------------

  if (-not $SkipTests) {
    Invoke-Step "cargo test" {
      $testFailed = $false
      foreach ($crate in $crateNames) {
        Write-Host "Testing $crate..."
        Invoke-LoggedCommand "cargo test -p $crate --all-features --no-fail-fast" -GroupOutput -DoNotExitOnFailedExitCode
        if ($LASTEXITCODE -ne 0) { $testFailed = $true }
      }

      if ($IncludeEmulatorTests) {
        Write-Host "`nRunning emulator-dependent tests..."
        foreach ($crate in $crateNames) {
          Write-Host "Emulator tests for $crate..."
          $env:RUSTFLAGS = '--cfg test_category="emulator"'
          Invoke-LoggedCommand "cargo test -p $crate --tests --no-fail-fast" -GroupOutput -DoNotExitOnFailedExitCode
          if ($LASTEXITCODE -ne 0) { $testFailed = $true }
          $env:RUSTFLAGS = $null
        }
      }

      if ($testFailed) { $global:LASTEXITCODE = 1 } else { $global:LASTEXITCODE = 0 }
    }
  }
  else {
    Write-Host "`nSkipping cargo test (--SkipTests)`n"
    $stepResults["cargo test"] = 'SKIPPED'
  }
}

# ---------------------------------------------------------------------------
# Step 5: Spell check (changed files only)
# ---------------------------------------------------------------------------

$spellCheckScript = ([System.IO.Path]::Combine($RepoRoot, 'eng', 'common', 'scripts', 'check-spelling-in-changed-files.ps1'))
if (Test-Path $spellCheckScript) {
  Invoke-Step "spell check" {
    Write-Host "Checking spelling in changed files..."
    & $spellCheckScript -TargetCommittish $BaseBranch -ExitWithError
    if ($LASTEXITCODE -ne 0) { $global:LASTEXITCODE = 1 } else { $global:LASTEXITCODE = 0 }
  }
}
else {
  LogWarning "Spell check script not found at $spellCheckScript"
  $stepResults["spell check"] = 'SKIPPED'
}

# ---------------------------------------------------------------------------
# Step 6: Markdown lint (if markdown files changed)
# ---------------------------------------------------------------------------

if ($changedMarkdown -and $changedMarkdown.Count -gt 0) {
  $lintMarkdownDir = ([System.IO.Path]::Combine($RepoRoot, '.github', 'skills', 'lint-markdown'))

  if (Test-Path ([System.IO.Path]::Combine($lintMarkdownDir, 'package.json'))) {
    Invoke-Step "markdown lint" {
      $mdLintFailed = $false

      # Install dependencies if needed
      if (-not (Test-Path ([System.IO.Path]::Combine($lintMarkdownDir, 'node_modules')))) {
        Write-Host "Installing markdownlint-cli2..."
        Push-Location $lintMarkdownDir
        Invoke-LoggedCommand "npm ci" -GroupOutput -DoNotExitOnFailedExitCode
        Pop-Location
        if ($LASTEXITCODE -ne 0) {
          LogWarning "Failed to install markdownlint-cli2"
          $mdLintFailed = $true
        }
      }

      if (-not $mdLintFailed) {
        Write-Host "Linting changed markdown files..."
        $mdFiles = $changedMarkdown | ForEach-Object { "`"$_`"" }
        $mdFileArgs = $mdFiles -join ' '
        Push-Location $RepoRoot
        Invoke-LoggedCommand "npm exec --prefix .github/skills/lint-markdown -- markdownlint-cli2 $mdFileArgs" -GroupOutput -DoNotExitOnFailedExitCode
        Pop-Location
        if ($LASTEXITCODE -ne 0) { $mdLintFailed = $true }
      }

      if ($mdLintFailed) { $global:LASTEXITCODE = 1 } else { $global:LASTEXITCODE = 0 }
    }
  }
  else {
    LogWarning "lint-markdown skill not found, skipping markdown lint"
    $stepResults["markdown lint"] = 'SKIPPED'
  }
}
else {
  Write-Host "`nNo markdown files changed, skipping markdown lint`n"
  $stepResults["markdown lint"] = 'SKIPPED'
}

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host " Validation Summary" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

$anyFailed = $false
foreach ($step in $stepResults.Keys) {
  $result = $stepResults[$step]
  switch ($result) {
    'PASSED'  { Write-Host "  [PASS]    $step" -ForegroundColor Green }
    'FAILED'  { Write-Host "  [FAIL]    $step" -ForegroundColor Red; $anyFailed = $true }
    'SKIPPED' { Write-Host "  [SKIP]    $step" -ForegroundColor Yellow }
  }
}

Write-Host ""

if ($anyFailed) {
  Write-Host "Validation completed with failures." -ForegroundColor Red
  exit 1
}
else {
  Write-Host "All validations passed." -ForegroundColor Green
  exit 0
}
