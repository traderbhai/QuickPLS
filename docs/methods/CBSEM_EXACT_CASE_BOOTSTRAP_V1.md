# Exact CFA case bootstrap v1

Status: release-qualified scoped Standard for the exact CFA identity below.
The separate Version 2.50 recursive structural bootstrap belongs to its own
General SEM Rank 3 cell and does not reinterpret this contract.

## Identity and supported scope

The registered capability cell is:

- capability: `smartpls.cbsem_bootstrapping`
- cell: `qpls3.cbsem.bootstrap`
- family identity: `cbsem_exact_case_bootstrap_v1`

One Recipe-v4 exact-CFA run may additionally carry one of these immutable
interval sidecar identities:

- percentile Type-7: base `cbsem_exact_case_bootstrap_v1`
- analytic studentized Type-7:
  `cbsem_exact_case_bootstrap_analytic_studentized_interval_v1`
- BCa Type-7: `cbsem_exact_case_bootstrap_bca_interval_v1`

The point ML capability remains the separate
`smartpls.cbsem / qpls3.cbsem.ml / cbsem_ml_v1` cell. Historical
`cbsem_bootstrap_v2` (schema 3, percentile-only) and
`cbsem_bootstrap_v1` results remain readable under their own identities and
cannot be promoted, relabeled, or used as exact-family evidence.

The current bootstrap scope is raw continuous, single-group, reflective CFA
with listwise deletion and the exact Recipe-v4 ML estimator. Each preplanned
case draw has one indexed attempt. There are no retries or replacement draws;
attempted equals usable plus failed, and every failed fit retains its index and
typed reason. Requested `B` is 500 through 10,000. The fixed confidence level
is 95%. The compiled runner applies the usable-count threshold and stores typed
unavailable inference when it is not met.

Analytic studentized and BCa inference use the bounded exact-CFA contract:
at most 180 complete cases, 9 modeled observed variables, 18 free-parameter
rows, 18 optimizer dimensions, and 12 workers. Both use a fixed two-sided
contract. Studentized inference uses expected-information standard errors for
the point fit and each successful outer refit. BCa uses the shared successful
outer ledger plus exactly one no-retry delete-one fit per complete case; one
delete-one failure makes BCa globally typed unavailable.

## Arithmetic and reproducibility

Percentile endpoints use the Type-7 empirical quantile. The bootstrap standard
error is the sample standard deviation of the usable estimates. Analytic
studentized endpoints use the reverse pivot
`theta_hat - Q(t*) * SE(theta_hat)`. BCa uses midrank bias correction, the
complete delete-one jackknife acceleration, adjusted normal probabilities, and
Type-7 final quantiles.

The seed and complete-case sampling frame define an indexed plan. Executing
different worker partitions preserves the indexed analytical payload. A
cancelled run commits no result; rerunning the same recipe and dataset
fingerprint reproduces the uninterrupted payload. Geometry and arithmetic are
invariant to reordering stored successful estimates after indices are restored;
reimporting rows in a different order creates a different dataset fingerprint
and is not claimed to produce byte-identical samples.

## Persistence and trust boundary

The current Standard desktop workspace emits Recipe-v4 with the exact
bootstrap cell, executes through the exact compiler/runner, and appends the
native canonical document through the method-scoped
`standard_exact_cbsem` schema-6 boundary. Reopen validation checks capability
and method identity, dataset/recipe/model bindings, indexed accounting,
ledgers, derived interval arithmetic, and stored checksums atomically.

QuickPLS project archives are unsigned local files. These checks detect
accidental changes, partial changes, identity drift, malformed payloads, and
uncoordinated tampering. They do not authenticate a trusted author and do not
claim detection of a malicious coordinated semantic rewrite that also replaces
every dependent value and unkeyed checksum. Raw refits are not replayed during
ordinary reopen. That exclusion is a truthful security boundary, not a defect
in the stated structural-integrity claim.

## Product route

Use the **Exact CB-SEM** model tab. Enable **Exact CFA bootstrap**, choose
500–10,000 refits and one of the three interval methods, then start the native
job. The ordinary Calculate dialog remains point-only for CB-SEM and directs
archived bootstrap settings to this workspace, preventing studentized or BCa
selection from being serialized through historical percentile-only schema 3.

The result surface exposes the exact point fit, attempted/usable/failed counts,
indexed failures, requested interval tables or typed unavailability, method
versions, seed, worker provenance, and dataset/recipe/model digests. The XLSX
action adapts that selected immutable canonical document directly, including a
run-provenance sheet and canonical notices; schema-6 append/reopen uses the same
selected document identity.

## Exclusions

Recursive structural SEM bootstrap is outside this exact CFA identity and is
available only through the separately registered bounded Rank 3 General SEM
cell. Multigroup bootstrap, ordinal/WLSMV,
robust or FIML estimation, weights, clustered/multilevel sampling, parametric
and Bollen-Stine bootstrap, custom confidence levels, arbitrary one-sided
studentized/BCa tests, automatic model modification, and cryptographic
trusted-origin authentication are outside this identity.

## Focused evidence

`validation/cbsem_exact_case_bootstrap_v1_contract_matrix.py` is a compact
independent frozen arithmetic/metamorphic fixture. It is not represented as a
coverage simulation or a substitute for ML-reference validation. Existing
exact-ML reference reports and exact-bootstrap engine tests cover the estimator,
indexed no-retry execution, typed failures, cancellation, and sidecar payloads.
`validation/cbsem_exact_case_bootstrap_v1_source_gate.py` binds those current
source paths to the capability, desktop, and archive bridge. The existing
release-qualified identity remains frozen independently of the additive Rank 3
cell.

The packaged offline gate is functional and method-scoped: the exact-CFA
workflow must complete, export, save, and reopen without an internet-dependent
QuickPLS service, while application/page evidence reports zero external
requests. The supervisor also preserves exact process-tree TCP samples as a
separate platform observation. Microsoft-managed WebView2 background egress is
recorded as `platform_background_egress_observed`; if observed,
`commercial_zero_egress_passed` is necessarily `false` but does not fail the
bounded method qualification. This evidence never substitutes for the separate
OS-enforced commercial zero-process-egress containment gate.
