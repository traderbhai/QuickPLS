# PLS-POS v1

Status: the bounded engine and its historical QuickPLS v1.2.2 audit exist. The QuickPLS 3 method-promotion factory nevertheless derives `absent` because the new independent-reference, simulation, strict-persistence, native/export, identity-bound audit, and installed-application evidence roles are empty. The old audit must not be presented as QuickPLS 3 release qualification.

`pls_pos_v1` is emitted from `AnalysisMethod::Predict` for a typed PLS-POS configuration or compatible `segment_count` metadata. The contract is deliberately narrower than unrestricted published PLS-POS: it is the frozen QuickPLS deterministic score-space routine below.

## Inputs and applicability

- The requested segment count is an integer from 2 through 5.
- At least one recursive structural path and 40 complete model observations are required.
- Deterministic starts are clamped to 1 through 50.
- Minimum segment share is clamped to 0.05 through 0.40. Each segment must contain at least `max(8, ceil(n * minimum_share))` observations.
- The ordinary PLS point model and every segment structural regression must be finite and estimable.

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, unsupported missing-data shapes, too-small samples, and singular segment fits are blocked. The requested segment count is fixed by the analyst; v1 does not estimate or recommend a population class count.

## Frozen QuickPLS algorithm

For each observation, QuickPLS creates a feature vector from:

1. every construct score; and
2. the product of source and target construct scores for every structural path.

Each nonconstant feature column is standardized. Initial centroids are selected deterministically from an ordering based on a stable weighted feature-row key. For each start, the routine performs twelve assignment/update iterations:

1. assign each row to the nearest centroid under squared Euclidean feature distance;
2. move rows deterministically from donor segments until the minimum-size guard is met;
3. recompute centroids; and
4. fit every endogenous construct on its direct predecessors within each segment.

The selected state is the lowest finite structural residual objective encountered across every start and iteration:

`J(G) = sum_g sum_t sum_(i in g) [eta_ti - mean_g(eta_t) - sum_(s -> t) beta_gst (eta_si - mean_g(eta_s))]^2`.

The descriptive improvement is `(J_pooled - J_selected) / J_pooled`. It is not a p value or proof of heterogeneity.

## Result and interpretation boundary

The payload reports stable memberships, segment paths and R-squared, the selected and pooled objectives, descriptive improvement, minimum segment share, size imbalance, maximum pairwise path separation, and objective history.

Segment labels are arbitrary. An in-sample objective improvement does not establish a stable population segment, generalization, causal heterogeneity, or the correct number of groups. Product and export wording must call this the bounded deterministic QuickPLS score-space contract. It must not claim that the centroid routine reproduces every objective and hill-climbing detail of Becker et al. (2013).

`pls_pos_bounded_v1` remains backward-compatible for older metadata using `pls_pos_segments = "2"`, but its archive identity cannot satisfy `pls_pos_v1` promotion evidence.

## QuickPLS 3 qualification contract

`validation/methods/pls_pos_v1.manifest.json` freezes the required evidence. The current `npm run qpls:pos:recovery`, `npm run qpls:v06:validate`, and `npm run qpls:promotion:pls-pos` outputs are useful legacy engine evidence, but they do not satisfy the new identity-bound roles.

Release qualification still requires an independent implementation of this exact QuickPLS routine, hand identities, two-to-five-segment recovery and homogeneous-null simulations, label-aware metamorphic tests, strict save/reopen validation, dedicated native setup/results/export coverage, current method audit output, and installed Windows acceptance.

## Scientific context

Becker, Rai, Ringle, and Volckner (2013), *Discovering Unobserved Heterogeneity in Structural Equation Models to Avert Validity Threats*, MIS Quarterly 37(3), 665-694, https://aisel.aisnet.org/misq/vol37/iss3/3/.
