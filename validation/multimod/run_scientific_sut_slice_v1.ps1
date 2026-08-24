[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("all", "mga", "fimix", "pos", "conditional", "causal")]
    [string]$Gate,

    [Parameter(Mandatory = $true)]
    [ValidateSet("development", "qualification")]
    [string]$Scale,

    [string]$Output = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$sutOutput = [System.IO.Path]::GetTempFileName()

try {
    Push-Location -LiteralPath $repositoryRoot
    try {
        & cargo run --quiet --locked -p qpls-runner --example multimod_scientific_probe_v1 -- `
            --gate $Gate `
            --scale $Scale `
            --output $sutOutput
        if ($LASTEXITCODE -ne 0) {
            $probeExitCode = $LASTEXITCODE
            [ordered]@{
                schema_version = 1
                gate_id = "qpls.multimod.scientific_sut_vs_reference.v1"
                requested_gate = $Gate
                scale = $Scale
                status = "sut_probe_error"
                failure_codes = @("MMQ.SUT.PROBE.EXIT_$probeExitCode")
            } | ConvertTo-Json -Depth 10
            exit $probeExitCode
        }

        $compareArguments = @(
            "validation/multimod/compare_scientific_sut_v1.py",
            "--sut-json", $sutOutput,
            "--gate", $Gate,
            "--expected-scale", $Scale
        )
        if (-not [string]::IsNullOrWhiteSpace($Output)) {
            $compareArguments += @("--output", [IO.Path]::GetFullPath($Output))
        }
        & python @compareArguments
        exit $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
}
finally {
    if (Test-Path -LiteralPath $sutOutput -PathType Leaf) {
        Remove-Item -LiteralPath $sutOutput -Force
    }
}
