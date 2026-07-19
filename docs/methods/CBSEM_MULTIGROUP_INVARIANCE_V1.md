# CB-SEM Multigroup and Invariance v1

`cbsem_multigroup_v1` and `cbsem_invariance_v1` are experimental QuickPLS v0.7 multigroup and invariance preview payloads.

## Scope

- Uses metadata `cbsem_group_column` to split raw data into observed groups.
- Reports group sizes, group fit summaries, and configural, metric, and scalar invariance preview rows.
- Reports chi-square difference, delta CFI, delta RMSEA, degrees-of-freedom deltas, warnings, and mean-structure requirements.

## Unsupported

Full constrained multigroup ML refits, exact equality-constraint optimization, covariance/correlation multigroup input without group matrices, partial invariance search, and publication-ready measurement-invariance claims are unsupported.

## Validation

`npm run qpls:cbsem:multigroup-reference` and `npm run qpls:v07:validate` write `validation/results/cbsem_v07_reference_report.json`.
