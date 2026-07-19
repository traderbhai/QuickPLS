# rho_A Reference Qualification

QuickPLS uses three separate forms of evidence for Dijkstra-Henseler rho_A.

## Executable Local Evidence

`rho_a_reference.py` is a standard-library-only Decimal implementation of the published equation. It does not import QuickPLS or a PLS package. It verifies the committed results for equal, unequal signed, improper-below-zero, and improper-above-one supplied matrix/weight cases.

```powershell
validation\.venv\Scripts\python.exe validation\rho_a_reference.py
cargo test -p qpls-assessment
```

The Rust suite additionally covers coordinate conversion for standardized, mean-centered, and unstandardized estimation; path and factor inner weighting; positive affine data changes; construct/indicator permutation; listwise missing rows; persisted-loading mismatch; constant/nonfinite inputs; zero score variance; zero off-diagonal denominator; applicability; and project tamper rejection.

## Executed cSEM Evidence

`rho_a_csem_reference.R` is a development-only runner for cSEM 0.6.1. It uses reflective common-factor syntax, obtains PLS-PM Mode A weights, evaluates Equation 3 independently from the empirical indicator correlation matrix, and requires agreement with cSEM's weighted empirical reliability calculation before writing output.

```powershell
$env:R_LIBS_USER="$PWD\validation\r-library"
$env:QPLS_RSCRIPT="C:\Users\mohd.naved\AppData\Local\Programs\R\R-4.6.1\bin\Rscript.exe"
Rscript --vanilla validation\rho_a_csem_reference.R `
  validation\fixtures\rho_a_reference.csv `
  validation\results\rho_a_csem_0_6_1.csv
cargo run -p qpls-cli -- run validation\fixtures\rho_a_reference.recipe.json `
  --data validation\fixtures\rho_a_reference.csv `
  --output validation\results\rho_a_quickpls_reference.json `
  --allow-experimental
python validation\rho_a_csem_compare.py
```

The current committed comparison in `results/rho_a_csem_comparison.json` passes against cSEM 0.6.1 with maximum absolute difference `4.440892098500626e-16`, below the `1e-6` external-engine tolerance. R and cSEM remain development-only validation tools and are not runtime dependencies of QuickPLS.

The primary-paper population example also remains pending because its authoritative input fixture and expected output are not yet committed.
