# Numerical Validation Policy

1. Freeze the published formula, preprocessing convention, stopping rule, and expected result before implementation.
2. Add hand-calculated fixtures for statistical primitives and simulated fixtures for estimators.
3. Compare deterministic estimates with two independent reference implementations where possible. The default deterministic tolerance is `1e-6`.
4. Add metamorphic checks for indicator ordering, positive affine scaling, construct ordering, thread count, and fixed-seed reruns.
5. Validate resampling by coverage, bias, type-I error, and fixed-seed reproducibility rather than matching another program's random stream.
6. Record fixture provenance, package versions, hardware, compiler, settings, and known differences.
7. Do not label a method validated while any release-blocking discrepancy remains unexplained.

GPL reference engines may be used by development scripts but are never linked into or distributed with QuickPLS. R, Rscript, cSEM, lavaan, plspm, and similar reference engines are validation-workstation dependencies only; QuickPLS itself must remain a fully offline desktop application with no R runtime requirement for end users.

If R is installed but not on PATH, the Python validation scripts auto-discover the local portable `Documents\PLS-Sem\dist-desktop\r-runtime` layout, common Windows R install locations, and the R registry entries. You can still override discovery explicitly:

```powershell
$env:QPLS_RSCRIPT="C:\Path\To\R\bin\Rscript.exe"
$env:QPLS_R="C:\Path\To\R\bin\R.exe"
python validation\external_reference_probe.py
```

The current workstation has R 4.6.1 outside PATH. For PLS-core and rho_A cSEM comparisons, run:

```powershell
npm run qpls:pls:csem
npm run qpls:pls:plspm
npm run qpls:pls:pca
npm run qpls:rho-a:csem
```
