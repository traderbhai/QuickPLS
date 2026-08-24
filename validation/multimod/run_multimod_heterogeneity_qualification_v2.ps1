[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Output,

    [ValidateSet("development", "qualification")]
    [string]$Scale = "qualification",

    [UInt64]$Seed = 42
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$rawReceipt = [System.IO.Path]::GetTempFileName()
$resolvedOutput = [System.IO.Path]::GetFullPath($Output)

try {
    Push-Location -LiteralPath $repositoryRoot
    try {
        & cargo run --quiet --locked -p qpls-runner --example multimod_heterogeneity_qualification_v2 -- `
            --output $rawReceipt `
            --scale $Scale `
            --seed $Seed
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }

        & python validation/multimod/compare_multimod_heterogeneity_qualification_v2.py `
            --input $rawReceipt `
            --output $resolvedOutput
        exit $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
}
finally {
    if (Test-Path -LiteralPath $rawReceipt -PathType Leaf) {
        Remove-Item -LiteralPath $rawReceipt -Force
    }
}
