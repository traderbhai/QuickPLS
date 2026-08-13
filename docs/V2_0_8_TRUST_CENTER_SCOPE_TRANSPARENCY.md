# QuickPLS v2.0.8 Trust Center And Scope Transparency

## Summary

v2.0.8 applies the QuickPLS 2.0 desktop visual contract to the Trust Center. The page now explains why a result can be trusted, what scope is validated, where the supporting evidence lives, and what QuickPLS deliberately does not claim.

This milestone is frontend/product-only. It does not change statistical engines, method formulas, analysis recipes, result schemas, project archives, validation tolerances, or numerical fingerprints.

## What Changed

- Added a v2 Trust Center hero focused on researcher confidence and scope transparency.
- Added current-method confidence details: method status wording, applicability, dataset fingerprint, saved runs, latest run seed, and latest run fingerprint.
- Added a validation artifact index linking the main supported-scope, compatibility, known-difference, promotion, and release documents.
- Added a method scope matrix that reuses the existing method applicability engine instead of duplicating method availability rules.
- Added explicit offline/runtime, validation-only dependency, SmartPLS non-equivalence, and export-scope boundaries.
- Added confidence cards for formula specification, independent references, deterministic tolerance, and offline provenance.
- Preserved all numerical backend behavior.

## Acceptance

- `npm run qpls:v208:trust-center` passes.
- `cargo run -p qpls-cli -- gate v2_0_8_trust_center_scope_transparency` reports no open or blocked gates.
- Version metadata uses `2.0.8`.
- Fresh installer, portable executable, and checksums can be generated through `npm run qpls:desktop:build-versioned`.

## Out Of Scope

- No estimator behavior changes.
- No result payload changes.
- No new method validation claims.
- No project schema or archive-format changes.
- No SmartPLS project import or undocumented equivalence claim.
