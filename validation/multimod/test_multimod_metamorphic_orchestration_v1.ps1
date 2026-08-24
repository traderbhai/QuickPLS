[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$wrapperPath = Join-Path $PSScriptRoot "run_multimod_metamorphic_qualification_v1.ps1"
$bindingPath = Join-Path $PSScriptRoot "multimod_gate_bindings_v1.json"

function Assert-Contract {
    param(
        [Parameter(Mandatory = $true)][bool]$Condition,
        [Parameter(Mandatory = $true)][string]$Message
    )
    if (-not $Condition) { throw $Message }
}

$tokens = $null
$parseErrors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
    $wrapperPath,
    [ref]$tokens,
    [ref]$parseErrors
)
Assert-Contract ($parseErrors.Count -eq 0) (
    "Metamorphic wrapper has PowerShell parse errors: " +
    (($parseErrors | ForEach-Object Message) -join "; ")
)

$wrapper = Get-Content -LiteralPath $wrapperPath -Raw -Encoding UTF8
Assert-Contract (-not $wrapper.ToLowerInvariant().Contains("cargo run")) `
    "Metamorphic cells must launch built executables directly."
Assert-Contract ($wrapper.Contains("target/release/examples")) `
    "Metamorphic qualification must execute optimized release producers."
Assert-Contract (-not $wrapper.Contains("target/debug/examples")) `
    "Metamorphic qualification must not execute unoptimized debug producers."
Assert-Contract ($wrapper.Contains('"build", "--release"')) `
    "Metamorphic qualification must build its producers with Cargo's release profile."
Assert-Contract ([regex]::Matches($wrapper, '-FileName\s+"cargo"').Count -eq 1) `
    "Metamorphic orchestration must contain exactly one bounded Cargo invocation."
Assert-Contract ($wrapper.Contains('$Job.Process.Kill($true)')) `
    "Metamorphic child cleanup must terminate the complete process tree."
Assert-Contract ($wrapper.Contains('[ValidateRange(1, 4)]')) `
    "Metamorphic process concurrency must be capped at four."
Assert-Contract ($wrapper.Contains('[ValidateRange(60, 1800)]')) `
    "Every scientific cell must have a maximum 1,800-second binding."
Assert-Contract ($wrapper.Contains('[ValidateRange(600, 6480)]')) `
    "Post-build scientific work must have a maximum 6,480-second binding."
Assert-Contract ($wrapper.Contains('[ValidateRange(600, 6600)]')) `
    "The wrapper must have a maximum 6,600-second binding."
$rootCall = $wrapper.IndexOf('Invoke-CellPhase -Name "baseline-root"', [StringComparison]::Ordinal)
$dependentCall = $wrapper.IndexOf('Invoke-CellPhase -Name "dependent-axis"', [StringComparison]::Ordinal)
Assert-Contract ($rootCall -ge 0 -and $dependentCall -gt $rootCall) `
    "All family baseline roots must complete before dependent axes start."

foreach ($producerWrapperName in @(
    "run_multimod_mga_qualification_v1.ps1",
    "run_multimod_heterogeneity_qualification_v2.ps1",
    "run_conditional_causal_raw_qualification_v1.ps1"
)) {
    $producerWrapperPath = Join-Path $PSScriptRoot $producerWrapperName
    $producerWrapper = Get-Content -LiteralPath $producerWrapperPath -Raw -Encoding UTF8
    Assert-Contract ($producerWrapper.Contains("target/release/examples")) `
        "$producerWrapperName must execute an optimized release producer."
    Assert-Contract (-not $producerWrapper.Contains("target/debug/examples")) `
        "$producerWrapperName must not execute an unoptimized debug producer."
    Assert-Contract ($producerWrapper.Contains('"build", "--release"')) `
        "$producerWrapperName must build its producer with Cargo's release profile."
}

$bindings = Get-Content -LiteralPath $bindingPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
function Get-GateStep {
    param(
        [Parameter(Mandatory = $true)][string]$GateId,
        [Parameter(Mandatory = $true)][string]$StepId
    )
    $gate = @($bindings.gates | Where-Object { [string]$_.gate_id -eq $GateId })
    Assert-Contract ($gate.Count -eq 1) "Expected exactly one $GateId gate binding."
    $step = @($gate[0].steps | Where-Object { [string]$_.step_id -eq $StepId })
    Assert-Contract ($step.Count -eq 1) "Expected exactly one $GateId/$StepId step binding."
    return $step[0]
}

$expected = @(
    [pscustomobject]@{ Gate = "mga.group_matrix"; Step = "mga_production_science"; Root = "{campaign_root}/execution-cache/mga-group-matrix" },
    [pscustomobject]@{ Gate = "fimix.recovery"; Step = "heterogeneity_production_science"; Root = "{campaign_root}/execution-cache/heterogeneity-recovery" },
    [pscustomobject]@{ Gate = "conditional.profile_matrix"; Step = "conditional_raw_qualification"; Root = "{campaign_root}/execution-cache/conditional-profile-matrix" },
    [pscustomobject]@{ Gate = "causal.known_targets"; Step = "causal_raw_qualification"; Root = "{campaign_root}/execution-cache/causal-known-targets" },
    [pscustomobject]@{ Gate = "metamorphic.global"; Step = "all_profile_production_metamorphic_matrix"; Root = "{campaign_root}/execution-cache/metamorphic-global" }
)
foreach ($row in $expected) {
    $step = Get-GateStep -GateId $row.Gate -StepId $row.Step
    Assert-Contract ([int]$step.maximum_seconds -eq 6600) `
        "$($row.Gate)/$($row.Step) must have a 6,600-second campaign cap."
    $arguments = @($step.arguments | ForEach-Object { [string]$_ })
    $workRootIndex = [Array]::IndexOf($arguments, "-WorkRoot")
    Assert-Contract ($workRootIndex -ge 0 -and $workRootIndex + 1 -lt $arguments.Count) `
        "$($row.Gate)/$($row.Step) must bind a stable WorkRoot."
    Assert-Contract ($arguments[$workRootIndex + 1] -ceq $row.Root) `
        "$($row.Gate)/$($row.Step) WorkRoot differs from its stable campaign cache."
}

$metamorphic = Get-GateStep -GateId "metamorphic.global" -StepId "all_profile_production_metamorphic_matrix"
$outputs = @($metamorphic.expected_outputs | ForEach-Object { [string]$_ })
Assert-Contract ($outputs -ccontains "{gate_output}/multimod-global-metamorphic.json") `
    "Metamorphic gate must publish the unchanged scientific report."
Assert-Contract ($outputs -ccontains "{gate_output}/multimod-global-metamorphic.json.execution-receipt.json") `
    "Metamorphic gate must publish the ordered execution receipt."

Write-Host "MultiMod metamorphic orchestration static contract passed."
