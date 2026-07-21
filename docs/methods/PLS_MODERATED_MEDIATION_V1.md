# Moderated Mediation v1

Status: validated for the documented QuickPLS v0.9.0-rc.1 supported moderated-mediation diagnostic scope. Broader moderated-mediation variants outside this contract remain unsupported.

`AnalysisMethod::ModeratedMediation` reuses the two-stage product-score interaction contract, then reports conditional indirect effects for mediated paths connected to the interaction. The current result reports `method_version = "pls_moderated_mediation_v1"` and stores a typed `moderated_mediation` payload.

Implemented contract:

- recipes must include at least one `ModelSpec.interactions` entry;
- path and factor weighting are supported; PCA is blocked for this preview;
- first-stage moderated mediation is detected when an interaction predicts a mediator and that mediator predicts a downstream target;
- second-stage moderated mediation is detected when an interaction predicts the final outcome and the moderated predictor has an upstream antecedent;
- conditional indirect effects are reported at standardized moderator scores `-1`, `0`, and `+1`;
- the index of moderated mediation is the product of the interaction coefficient and the unmoderated path on the other mediation stage.

Validation evidence:

- `npm run qpls:moderated-mediation:reference` writes `validation/results/moderated_mediation_reference_report.json`.
- The reference script independently standardizes the single-item fixture, computes the two-stage product score, solves the structural regressions, and compares conditional indirect effects plus the moderated mediation index within `1e-10`.
- Current observed reference delta is `4.67e-14`.
- The same script proves recipes without interaction terms are rejected with `moderated_mediation.interaction_required`.

Publication status: validated for the documented QuickPLS v1.2.3 two-stage conditional indirect-effect diagnostic scope. The full Hayes PROCESS catalogue and unsupported moderated-mediation variants remain experimental or unsupported.
