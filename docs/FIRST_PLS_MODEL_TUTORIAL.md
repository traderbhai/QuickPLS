# First PLS Model Tutorial

This tutorial uses the bundled simple reflective fixture.

## Dataset

The fixture is:

```text
validation/fixtures/simple_reflective.csv
```

It is a compact deterministic dataset for smoke testing the PLS workflow.

## Steps

1. Open QuickPLS.
2. Click `New`.
3. Click `Import` and select `validation/fixtures/simple_reflective.csv`.
4. Create construct `x` with indicators `x1` and `x2`.
5. Create construct `y` with indicators `y1` and `y2`.
6. Draw a structural path from `x` to `y`.
7. Choose PLS-SEM / PLS-PM.
8. Click `Run`.
9. Select the completed saved run.
10. Confirm that diagram estimates appear only after the run is selected.
11. Open Reports and export CSV, HTML, XLSX, or SVG.

## CLI Equivalent

```powershell
cargo run -p qpls-cli -- run validation/fixtures/simple_reflective.recipe.json --data validation/fixtures/simple_reflective.csv --output validation/results/tutorial_simple_reflective.json --allow-experimental
cargo run -p qpls-cli -- export validation/results/tutorial_simple_reflective.json --format csv --output validation/results/tutorial_simple_reflective.csv
```

## Expected Behavior

QuickPLS should produce a completed run with loadings, path coefficient, R², effects, provenance, and warnings scoped to the documented method contract.

