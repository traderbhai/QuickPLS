$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$python = Join-Path $root "validation\.venv\Scripts\python.exe"

if (-not (Test-Path $python)) {
  throw "validation Python venv not found at $python"
}

& $python (Join-Path $root "validation\pls_pca_reference.py")
& $python (Join-Path $root "validation\pls_pca_compare.py")
