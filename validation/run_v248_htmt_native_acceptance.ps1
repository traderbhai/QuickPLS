param(
    [Parameter(Mandatory = $true)]
    [string]$QualificationReport,

    [switch]$Admit
)

$arguments = @(
    "validation/htmt_packaged_acceptance.py",
    "--qualification-report",
    $QualificationReport
)
if ($Admit) {
    $arguments += "--admit"
}

& python @arguments
exit $LASTEXITCODE
