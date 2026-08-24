[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptPaths = @(
    (Join-Path $PSScriptRoot "package_multimod_candidate_v1.ps1"),
    (Join-Path $PSScriptRoot "invoke_multimod_gate_v1.ps1"),
    (Join-Path $PSScriptRoot "..\run_v256_multimod_qualification.ps1")
)

function Assert-Contract {
    param(
        [Parameter(Mandatory = $true)][bool]$Condition,
        [Parameter(Mandatory = $true)][string]$Message
    )
    if (-not $Condition) { throw $Message }
}

$asts = @{}
foreach ($scriptPath in $scriptPaths) {
    $tokens = $null
    $parseErrors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile(
        $scriptPath,
        [ref]$tokens,
        [ref]$parseErrors
    )
    Assert-Contract ($parseErrors.Count -eq 0) (
        "$scriptPath has PowerShell parse errors: " +
        (($parseErrors | ForEach-Object Message) -join "; ")
    )
    $startProcessCommands = @(
        $ast.FindAll(
            {
                param($node)
                $node -is [Management.Automation.Language.CommandAst] -and
                    $node.GetCommandName() -ceq "Start-Process"
            },
            $true
        )
    )
    Assert-Contract ($startProcessCommands.Count -eq 0) `
        "$scriptPath must not launch reviewed commands through Start-Process argument flattening."
    $asts[$scriptPath] = $ast
}

$nodeCommand = Get-Command "node.exe" -ErrorAction Stop
$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$fixtureRoot = Join-Path $temporaryBase ("qpls multimod argv fixture " + [Guid]::NewGuid().ToString("N"))
$resolvedFixtureRoot = [IO.Path]::GetFullPath($fixtureRoot)
Assert-Contract (($resolvedFixtureRoot + [IO.Path]::DirectorySeparatorChar).StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) `
    "Process argv fixture escaped the system temporary directory."

$savedCapture = [Environment]::GetEnvironmentVariable("QPLS_ARGV_CAPTURE", "Process")
try {
    $shimDirectory = Join-Path $resolvedFixtureRoot "tool chain"
    $cliDirectory = Join-Path $shimDirectory "node_modules\npm\bin"
    New-Item -ItemType Directory -Path $cliDirectory -Force | Out-Null
    $shimPath = Join-Path $shimDirectory "npm.cmd"
    $cliPath = Join-Path $cliDirectory "npm-cli.js"
    [IO.File]::WriteAllText(
        $shimPath,
        "@echo off`r`nexit /b 99`r`n",
        [Text.ASCIIEncoding]::new()
    )
    [IO.File]::WriteAllText(
        $cliPath,
        'require("node:fs").writeFileSync(process.env.QPLS_ARGV_CAPTURE, JSON.stringify(process.argv.slice(2)), "utf8");',
        [Text.UTF8Encoding]::new($false)
    )
    $expectedArguments = @(
        "alpha beta",
        '{"build":{"beforeBuildCommand":""}}'
    )
    $expectedJson = $expectedArguments | ConvertTo-Json -Compress

    foreach ($helperScript in $scriptPaths[0..1]) {
        $definition = @(
            $asts[$helperScript].FindAll(
                {
                    param($node)
                    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and
                        $node.Name -ceq "Resolve-ExactProcessLaunch"
                },
                $true
            )
        )
        Assert-Contract ($definition.Count -eq 1) `
            "$helperScript must define exactly one Resolve-ExactProcessLaunch helper."
        Invoke-Expression $definition[0].Extent.Text

        $capturePath = Join-Path $resolvedFixtureRoot (([IO.Path]::GetFileNameWithoutExtension($helperScript)) + ".json")
        [Environment]::SetEnvironmentVariable("QPLS_ARGV_CAPTURE", $capturePath, "Process")
        $launch = Resolve-ExactProcessLaunch -Executable $shimPath -Arguments $expectedArguments
        Assert-Contract ([string]$launch.LaunchKind -ceq "node_cli_argument_list") `
            "$helperScript did not bypass the npm.cmd parser."
        Assert-Contract ([IO.Path]::GetFullPath([string]$launch.FileName).Equals(
            [IO.Path]::GetFullPath([string]$nodeCommand.Source),
            [StringComparison]::OrdinalIgnoreCase
        )) "$helperScript did not resolve npm through node.exe."
        Assert-Contract ([string]$launch.Arguments[0] -ceq $cliPath) `
            "$helperScript did not preserve the exact npm CLI entry point."

        $startInfo = [Diagnostics.ProcessStartInfo]::new()
        $startInfo.FileName = [string]$launch.FileName
        $startInfo.UseShellExecute = $false
        $startInfo.CreateNoWindow = $true
        foreach ($argument in @($launch.Arguments)) {
            [void]$startInfo.ArgumentList.Add([string]$argument)
        }
        $process = [Diagnostics.Process]::new()
        $process.StartInfo = $startInfo
        try {
            [void]$process.Start()
            Assert-Contract ($process.WaitForExit(10000)) `
                "$helperScript argv fixture did not exit within ten seconds."
            Assert-Contract ([int]$process.ExitCode -eq 0) `
                "$helperScript executed the failing npm.cmd shim instead of npm-cli.js."
        }
        finally {
            if (-not $process.HasExited) { $process.Kill($true) }
            $process.Dispose()
        }
        Assert-Contract (Test-Path -LiteralPath $capturePath -PathType Leaf) `
            "$helperScript did not produce the argv capture."
        $capturedJson = Get-Content -LiteralPath $capturePath -Raw -Encoding UTF8
        Assert-Contract ($capturedJson.Equals($expectedJson, [StringComparison]::Ordinal)) `
            "$helperScript changed whitespace/JSON argv boundaries: $capturedJson"
    }
}
finally {
    [Environment]::SetEnvironmentVariable("QPLS_ARGV_CAPTURE", $savedCapture, "Process")
    if (Test-Path -LiteralPath $resolvedFixtureRoot) {
        $verifiedFixtureRoot = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $resolvedFixtureRoot).Path)
        Assert-Contract (($verifiedFixtureRoot + [IO.Path]::DirectorySeparatorChar).StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) `
            "Refusing to remove a process argv fixture outside the system temporary directory."
        Remove-Item -LiteralPath $verifiedFixtureRoot -Recurse -Force
    }
}

Write-Host "MultiMod Windows exact-argv fixture passed."
