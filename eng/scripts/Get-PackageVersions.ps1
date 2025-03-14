#!/usr/bin/env pwsh
#Requires -Version 7.0

[CmdletBinding()]
param(
  [switch]$AsDependencyTable
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot '..' 'common' 'scripts' 'common.ps1')

$metadata = cargo metadata --format-version 1 --no-deps --all-features | ConvertFrom-Json -AsHashtable
$packages = $metadata.packages
foreach ($package in $packages) {
  try {
    $name = $package.name
    $resp = Invoke-WebRequest "https://index.crates.io/$($name.Substring(0,2))/$($name.Substring(2,2))/$name"
    $packageVersions = $resp.Content.Trim().Split("`n") | ConvertFrom-Json | Select-Object -ExpandProperty vers
    $package.indexVersion = $packageVersions | Sort-Object { [AzureEngSemanticVersion]::ParseVersionString($_) } | Select-Object -Last 1
  }
  catch {
    if ($_.Exception.Response.StatusCode -eq 404) {
      $package.indexVersion = $null
    }
    else {
      throw
    }
  }

  $package.publish = $null -eq $package.publish

  foreach ($dependency in $package.dependencies) {
    $dependencyPackage = $packages | Where-Object -Property name -EQ -Value $dependency.name | Select-Object -First 1

    if ($dependencyPackage) {
      $ordered = [ordered]@{
        dependant    = $package.name
        pathVersion  = $dependencyPackage.version
        indexVersion = $dependencyPackage.indexVersion
      }

      foreach ($key in $dependency.Keys) {
        $ordered[$key] = $dependency[$key]
      }

      $dependencyPackage.dependantPackages ??= @()
      $package.packageDependencies ??= @()

      $dependencyPackage.dependantPackages += [hashtable]$ordered
      $package.packageDependencies += [hashtable]$ordered
    }
  }
}

$packages | Select-Object name, version, publish, indexVersion, packageDependencies, dependantPackages
