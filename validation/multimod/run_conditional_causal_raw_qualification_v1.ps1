[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("conditional", "causal")]
    [string]$Family,

    [Parameter(Mandatory = $true)]
    [ValidateSet("development", "qualification")]
    [string]$Scale,

    [Parameter(Mandatory = $true)]
    [string]$Output
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$outputPath = [System.IO.Path]::GetFullPath($Output)
$outputDirectory = [System.IO.Path]::GetDirectoryName($outputPath)
$outputStem = [System.IO.Path]::GetFileNameWithoutExtension($outputPath)
$producerOutput = Join-Path $outputDirectory "$outputStem.producer.json"
$example = if ($Family -eq "conditional") {
    "multimod_conditional_qualification_v1"
}
else {
    "multimod_causal_qualification_v1"
}

[System.IO.Directory]::CreateDirectory($outputDirectory) | Out-Null

Push-Location -LiteralPath $repositoryRoot
try {
    & cargo run --quiet --locked -p qpls-runner --example $example -- `
        --scale $Scale `
        --output $producerOutput
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    & python validation/multimod/verify_conditional_causal_raw_qualification_v1.py `
        --family $Family `
        --report $producerOutput `
        --expected-scale $Scale `
        --output $outputPath
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
