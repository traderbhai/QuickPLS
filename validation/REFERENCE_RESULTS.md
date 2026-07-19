# PLS-PM Reference Results

Fixture: `fixtures/simple_reflective.csv`

Model: reflective Mode A blocks `x = {x1, x2}` and `y = {y1, y2}`, with `x -> y`, per-indicator sample standardization, path inner weighting, tolerance `1e-7` or stricter.

| Estimate | Python plspm 0.5.7 | R cSEM 0.6.1 | QuickPLS tolerance |
| --- | ---: | ---: | ---: |
| x -> y | 0.983378918793432 | 0.9833789 | 1e-6 |
| loading x1 | 0.986495429512647 | 0.9864954 | 1e-6 |
| loading x2 | 0.984698236524415 | 0.9846982 | 1e-6 |
| loading y1 | 0.995439694535406 | 0.9954397 | 1e-6 |
| loading y2 | 0.995656444624731 | 0.9956564 | 1e-6 |
| weight x1 | sample/population normalization differs | 0.5230179 | 1e-6 |
| weight x2 | sample/population normalization differs | 0.4915670 | 1e-6 |
| weight y1 | sample/population normalization differs | 0.4961349 | 1e-6 |
| weight y2 | sample/population normalization differs | 0.5083356 | 1e-6 |
| Mode B x -> y | not used | 0.9984476 | 1e-6 |
| Mode B weight x1 | not used | 0.9931025 | 1e-6 |
| Mode B weight x2 | not used | 0.007312422 | 1e-6 |
| PCA x -> y | not used | 0.9823003 | 1e-6 |

Python `plspm` internally uses a global scale when `scaled=True`; the runner pre-standardizes each indicator and uses `scaled=False` to align preprocessing. Its final weights use a population-variance correction, so path coefficients and loadings are compared while weight normalization is compared with cSEM. GPL tools and their installed libraries are development-only and excluded from distribution.

`run_pls_csem.ps1` now writes `results/pls_csem_0_6_1.csv` and `results/pls_csem_comparison.json`. The comparison currently passes for path Mode A, Mode B, factor weighting, and PCA with maximum absolute differences below `1e-6`.

`run_pls_plspm.ps1` writes `results/pls_plspm_0_5_7.json` and `results/pls_plspm_comparison.json` from the isolated validation venv. The comparison currently passes for path Mode A, Mode B, and factor loadings/paths with maximum absolute differences below `1e-6`. Outer weights are excluded because python-plspm uses a different normalization convention.

`run_pls_pca_numpy.ps1` writes `results/pls_pca_numpy_reference.json` and `results/pls_pca_numpy_comparison.json` from an independent NumPy covariance eigensystem implementation. The comparison currently passes for PCA paths, loadings, and weights with maximum absolute difference `1.1102230246251565e-16`.

## rho_A Equation Reference

`rho_a_reference.py` independently evaluates the published Dijkstra-Henseler equation with 50-digit Decimal arithmetic for supplied matrices and weights. Its committed output in `results/rho_a_reference.json` agrees with the Rust implementation within `1e-14` for equal, unequal signed, improper-below-zero, and improper-above-one cases.

The reflective cSEM 0.6.1 runner in `rho_a_csem_reference.R` has been executed with the development-only R library in `validation/r-library`. `results/rho_a_csem_comparison.json` compares QuickPLS against cSEM/manual rho_A on `fixtures/rho_a_reference.csv`; the maximum absolute difference is `4.440892098500626e-16`, below the `1e-6` external-engine tolerance. Primary-paper agreement remains required before rho_A validation.
