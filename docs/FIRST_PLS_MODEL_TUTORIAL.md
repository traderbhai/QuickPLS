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
2. Choose `New`.
3. Open `Data` and import `validation/fixtures/simple_reflective.csv`.
4. Open `Model`.
5. Create construct `x` with indicators `x1` and `x2`.
6. Create construct `y` with indicators `y1` and `y2`.
7. Draw a structural path from `x` to `y`.
8. Choose `Calculate`.
9. Select `PLS path modeling core` from the compatible Standard methods and start the calculation.
10. QuickPLS opens `Results` after completion; select the completed saved run if more than one is present.
11. Confirm that diagram estimates appear only after the compatible run is selected.
12. Choose `Export` and save one of the formats available for that result.

## CLI Equivalent

```powershell
cargo run -p qpls-cli -- run validation/fixtures/simple_reflective.recipe.json --data validation/fixtures/simple_reflective.csv --output validation/results/tutorial_simple_reflective.json --allow-experimental
cargo run -p qpls-cli -- export validation/results/tutorial_simple_reflective.json --format csv --output validation/results/tutorial_simple_reflective.csv
```

## Expected Behavior

QuickPLS should produce a completed run with loadings, path coefficient, R², effects, provenance, and warnings scoped to the documented method contract.
