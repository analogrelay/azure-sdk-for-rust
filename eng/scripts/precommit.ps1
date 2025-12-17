#!/usr/bin/env pwsh
# Pre-commit script to run various quick checks before committing code.
# Optional, but highly recommended to avoid PR churn due to trivial CI issues.

$ErrorActionPreference = "Stop"

$repoRoot = git rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to find repository root"
    exit 1
}

Push-Location $repoRoot
try {
    # Clippy the whole workspace
    Write-Host "Running cargo clippy..." -ForegroundColor Cyan
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Clippy failed"
        exit $LASTEXITCODE
    }

    # Run fmt check
    Write-Host "Running cargo fmt..." -ForegroundColor Cyan
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Formatting check failed"
        exit $LASTEXITCODE
    }

    # Spell check
    $npxExists = Get-Command npx -ErrorAction SilentlyContinue
    if ($null -eq $npxExists) {
        Write-Warning "NodeJS is not installed. Skipping cspell."
    } else {
        Write-Host "Running cspell..." -ForegroundColor Cyan
        npx cspell lint --config ./.vscode/cspell.json "**/*"
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Spell check failed"
            exit $LASTEXITCODE
        }
    }

    # Run tests
    Write-Host "Running cargo test..." -ForegroundColor Cyan
    cargo test --workspace --all-features
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Tests failed"
        exit $LASTEXITCODE
    }

    Write-Host "All checks passed!" -ForegroundColor Green
} finally {
    Pop-Location
}
