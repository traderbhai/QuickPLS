# QuickPLS Method Promotion Criteria

This document defines when a QuickPLS calculation can move from experimental preview output to researcher-ready validated output.

Promotion is method-specific. A release can be stable while a calculation family remains experimental for broader or unsupported shapes. A method is validated only for the exact scope documented in its method specification, compatibility matrix, known-differences register, and validation artifacts.

## Required Evidence

Each promoted method must have:

- A frozen method specification with equations, defaults, preprocessing, sign conventions, convergence rules, missing-data behavior, output definitions, warnings, citations, and unsupported cases.
- Formula-level unit tests and matrix/property tests for deterministic components.
- At least two independent references where feasible. Acceptable references include published examples, hand-calculated fixtures, R engines, Python engines, NumPy/SciPy/statsmodels-style independent implementations, and primary-paper equations.
- Simulation evidence appropriate to the method: recovery, bias, coverage, Type-I error, power, false-positive containment, and failure behavior.
- Metamorphic tests for row order, construct order, indicator order, positive affine transformations, sign conventions, group splitting, and thread-count changes where applicable.
- Edge-case diagnostics for missing data, constants, collinearity, singular matrices, non-normal data, small samples, high-dimensional data, non-convergence, inadmissible estimates, unsupported recipes, and invalid constraints.
- GUI and CLI parity for the same recipe, seed, data fingerprint, method version, and settings.
- Export parity for CSV, HTML, XLSX, SVG/report tables where applicable.
- Reproducibility evidence for stochastic methods: fixed seed equality, indexed streams, stable replicate ordering, worker-count invariance, and deterministic aggregation.
- Performance evidence for the documented supported scope, plus explicit separation between quick smoke benchmarks and maximum benchmark profiles.
- Known-difference documentation for any reference-engine convention difference.
- Product labels and warnings that match the promoted scope.

## Status Rules

`experimental` means the method may be implemented and may have useful evidence, but the broader calculation claim is not yet qualified for unrestricted publication use.

`validated` means the method is researcher-ready only for the documented supported scope. It does not imply SmartPLS equivalence, undocumented behavior matching, or support for excluded model shapes.

Unsupported or post-v1 features must remain blocked, hidden, or visibly watermarked.

## Initial Promotion Order

Promote lower-risk, high-evidence methods first:

1. PLS core stable run envelope.
2. Assessment metrics.
3. Inference/resampling for documented PLS settings.
4. Standalone PCA and OLS regression.
5. Mediation, moderation, PLSc, WPLS, IPMA, PLSpredict, and NCA.
6. MICOM, MGA, higher-order constructs, nonlinear/endogeneity diagnostics, logistic regression, and PROCESS-style workflows.
7. CB-SEM/CFA broader ML scope, GSCA, FIMIX-PLS, and PLS-POS.

Any method with unexplained deterministic disagreement above `1e-6` remains experimental until resolved or documented as a justified known difference.
