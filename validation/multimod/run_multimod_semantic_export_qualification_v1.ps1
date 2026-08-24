[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Output
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$outputPath = [IO.Path]::GetFullPath($Output)
$outputParent = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Path $outputParent -Force | Out-Null
if (Test-Path -LiteralPath $outputPath -PathType Leaf) {
    Remove-Item -LiteralPath $outputPath -Force
}

$previous = $env:QPLS_MULTIMOD_EXPORT_REPORT
try {
    $env:QPLS_MULTIMOD_EXPORT_REPORT = $outputPath
    Push-Location -LiteralPath $repositoryRoot
    try {
        & npx.cmd --no-install vitest run `
            src/domain/multimodSemanticExportQualificationV1.test.ts `
            src/services/canonicalResultExportPublicationV2Service.test.ts
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
    finally {
        Pop-Location
    }
}
finally {
    $env:QPLS_MULTIMOD_EXPORT_REPORT = $previous
}

if (-not (Test-Path -LiteralPath $outputPath -PathType Leaf)) {
    throw "The MultiMod semantic-export qualification test did not publish its report."
}
