$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

& python (Join-Path $root "validation\run_studentized_qualification_shards.py") @args
