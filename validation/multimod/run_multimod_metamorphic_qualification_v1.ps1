[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Output
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$outputPath = [IO.Path]::GetFullPath($Output)
$runDirectory = Join-Path (Split-Path -Parent $outputPath) "metamorphic-production-runs"
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null

$saved = [ordered]@{
    Metamorphism = $env:QPLS_MULTIMOD_METAMORPHISM_V1
    SignColumns = $env:QPLS_MULTIMOD_SIGN_COLUMNS_V1
    Workers = $env:QPLS_MULTIMOD_WORKERS_V1
    Compact = $env:QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1
}

function Invoke-MultiModMetamorphicProducer {
    param(
        [Parameter(Mandatory = $true)][string]$Family,
        [Parameter(Mandatory = $true)][string]$Example,
        [Parameter(Mandatory = $true)][string]$Axis,
        [string]$SignColumns = "",
        [int]$Workers = 1
    )
    $destination = Join-Path $runDirectory "$Family-$Axis.json"
    $env:QPLS_MULTIMOD_METAMORPHISM_V1 = $Axis
    $env:QPLS_MULTIMOD_SIGN_COLUMNS_V1 = $SignColumns
    $env:QPLS_MULTIMOD_WORKERS_V1 = [string]$Workers
    $env:QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1 = "1"
    & cargo run --quiet --locked -p qpls-runner --example $Example -- `
        --scale development `
        --output $destination
    if ($LASTEXITCODE -ne 0) {
        throw "MultiMod metamorphic producer $Family/$Axis exited with $LASTEXITCODE."
    }
    if (-not (Test-Path -LiteralPath $destination -PathType Leaf)) {
        throw "MultiMod metamorphic producer $Family/$Axis omitted its report."
    }
}

try {
    Push-Location -LiteralPath $repositoryRoot
    try {
        $sharedAxes = @(
            @{ Axis = "baseline"; Workers = 1 },
            @{ Axis = "seed_repeat"; Workers = 1 },
            @{ Axis = "row_reverse"; Workers = 1 },
            @{ Axis = "input_column_reverse"; Workers = 1 },
            @{ Axis = "declaration_reverse"; Workers = 1 },
            @{ Axis = "worker_parallel"; Workers = 4 }
        )
        foreach ($family in @(
            @{ Id = "mga"; Example = "multimod_mga_qualification_v1" },
            @{ Id = "heterogeneity"; Example = "multimod_heterogeneity_qualification_v2" },
            @{ Id = "conditional"; Example = "multimod_conditional_qualification_v1" },
            @{ Id = "causal"; Example = "multimod_causal_qualification_v1" }
        )) {
            foreach ($entry in $sharedAxes) {
                Invoke-MultiModMetamorphicProducer `
                    -Family $family.Id `
                    -Example $family.Example `
                    -Axis $entry.Axis `
                    -Workers $entry.Workers
            }
        }
        Invoke-MultiModMetamorphicProducer `
            -Family "causal" `
            -Example "multimod_causal_qualification_v1" `
            -Axis "sign_reverse" `
            -SignColumns "c" `
            -Workers 1

        & python validation/multimod/verify_multimod_metamorphic_qualification_v1.py `
            --input-directory $runDirectory `
            --capability-index validation/multimod/multimod_capability_index_v1.json `
            --repository-root $repositoryRoot `
            --output $outputPath
        exit $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
}
finally {
    $env:QPLS_MULTIMOD_METAMORPHISM_V1 = $saved.Metamorphism
    $env:QPLS_MULTIMOD_SIGN_COLUMNS_V1 = $saved.SignColumns
    $env:QPLS_MULTIMOD_WORKERS_V1 = $saved.Workers
    $env:QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1 = $saved.Compact
}
