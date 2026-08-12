# NCA v1 legacy compatibility

Status: superseded; archive-readable only.

`nca_v1` is retained so existing `.qpls` archives can be opened without silently rewriting historical result values. It is not the current executable NCA method and must not be used as promotion or scientific-validation evidence.

## Why it was superseded

The former implementation did not match the documented CE-FDH and CR-FDH definitions:

- its CE frontier selected outcomes from observations at or above each X value instead of building cumulative record highs while scanning X in ascending order; and
- its CR result approximated the frontier from adjacent raw outcomes instead of fitting ordinary least squares through the CE-FDH peers.

Those semantics are corrected under the separately versioned [`nca_v2`](NCA_V2.md) contract. QuickPLS does not reinterpret a stored `nca_v1` payload as v2.

## Compatibility behavior

- A structurally valid legacy `nca_v1` result may be loaded, saved, and reopened.
- Loading adds the warning diagnostic `nca.legacy_method_version`.
- Missing v2-only fields remain missing/defaulted; QuickPLS does not manufacture v2 peers, line parameters, or bottleneck states for the legacy result.
- New calculations emit `nca_v2` only.
- Any analysis that will be interpreted, reported, or compared should be rerun with NCA v2 from the original data and recipe choices.

This compatibility policy preserves provenance. It does not validate the earlier numerical result.
