# QuickPLS 2.55 — Calculate and Method Evidence Gate

Status: consolidated source gate complete; candidate and publication gates pending.

The formal first consolidated pass recorded the finite correction batch and the byte-identical final pass succeeded 14/14. Version authorities are now **2.55.0** for the provenance-bound candidate phase. Portable and installed smoke, actual Windows 200% evidence, final evidence collection, and publication remain mandatory; this checkpoint does not claim a downloadable 2.55 release or promote a capability.

## Scope

The 2.55 gate covers the shared Calculate dialog, method-specific eligibility and settings, evidence rebaselining, and current presentation evidence. It preserves the public Calculate catalogue at exactly 18 methods and does not repeat the already-qualified scientific matrices.

## Evidence Sources

- `validation/v255_regression_rebaseline.json` records all 17 historical failed-contract dispositions. Every replacement is an interaction route, not a source-string assertion.
- `validation/v255_rebased_interaction_contracts.mjs` records each REB-001 through REB-017 separately. Component/domain/service/governance rows bind one exact passing assertion from the machine-readable Vitest report; browser rows execute a controlled fixture and capture a screenshot. A missing fixture or assertion fails rather than inheriting coverage from method selection.
- `validation/v255_method_evidence_matrix.json` records setup cases, result families, imports, exports, persistence, accessibility, and observability coverage. Version 2 requires one evidence declaration for every setup case plus one separate controlled Calculate capture for each public method; a generic capture never substitutes for a scientific eligibility case.
- `validation/v255_named_evidence_index.json` freezes the exact 29 cross-method and 26 specialized-result observations independently of the matrix declarations. Its rows remain pending during source work and are curated with PNG/receipt members only after the candidate evidence bundle exists.
- `validation/v255_named_evidence_verifier.py` is the fail-closed publication authority for those 55 observations. It verifies safe ZIP paths, member SHA-256 values, PNG signatures, passing 2.55 JSON receipts, and an exact per-case JSON-pointer binding, then emits a report bound to the matrix, index, bundle manifest, and ZIP hashes.
- `validation/v255_frozen_result_archive_index.json` is the only authority for reusing completed Result evidence. Its version 3 evidence entries must exactly cover every declared Result family and independently bind archive, screenshot, and parsed receipt to a canonical result identity.
- `validation/v255_evidence_bundle_manifest.json` binds the collected binary evidence ZIP by SHA-256. The ZIP is a GitHub-release attachment, not an ignored local directory referenced from source history.
- `validation/v255_method_evidence_crawler.mjs` executes setup fixtures serially and records every setup case. It reuses completed archives for unchanged Result presentation only; it does not recompute an engine merely to recreate a screenshot.
- `validation/run_v255_unsigned_candidate_build.ps1` is the only candidate-build entry point. It requires a clean 2.55 source commit, uses a fresh target, runs locked NSIS-only Tauri packaging with `CARGO_INCREMENTAL=0`, watches the exact build process tree, and binds source tree/manifest, version authorities, disk policy, logs, and copied artifact hashes in the release-artifact report.
- `validation/run_v255_isolated_install.ps1` installs the exact report-bound NSIS setup into a brand-new destination and records setup, portable, and installed executable identity. A copied portable executable never counts as installed evidence.
- `validation/run_v255_cross_method_candidate_smoke.ps1` is invoked by the portable packaged pass. It supplies the native import/export, recovery/migration/read-only, unsaved-close, actual DPI=192, isolated-CDP, and PID-cleanup observations through a separate 17-case manifest and one source/candidate-bound driver report.

## Required Release Sequence

1. Finish T13–T16 product work:
   - T13: model-aware layout, routing, editable bends/anchors, and full PROCESS preview;
   - T14: responsive shared Calculate presentation;
   - T15: scientific eligibility and blocker priority;
   - T16: complete method-specific settings and fixed-policy disclosure.
2. Commit a clean 2.54.0 source-gate baseline with the 64 setup declarations, 18 Calculate-capture declarations, reusable-archive inventory, and fail-closed evidence indexes.
3. Run `validation/run_v255_consolidated_diagnostics.ps1` once. Its pre-candidate source gate verifies setup evidence plus the curated inventory of 17 hashable reusable archives and the one declared later packaged run; it does not falsely require a final ZIP before the unsigned candidate exists. It records every failure without stopping at the first ordinary command failure, emits `v255_full_vitest.json`, and requires all 17 rebased records at its final stage.
   It reserves 2.5 GiB for each Rust/full-Vitest step, 1.0 GiB for typecheck, 1.5 GiB for frontend build, and 0.5 GiB for browser crawls; C: and D: must remain strictly above 20 GiB. A gate-owned PID watcher stops only its own launched process tree at 20.25 GiB.
4. Correct the collected failure batch.
5. Run the identical diagnostic script once more.
6. Bump and commit only the approved 2.55 version authorities. The 2.55 commit must descend from the green source-gate commit and contain no post-gate product-code change.
7. Build one unsigned Windows candidate from that clean 2.55.0 commit with `run_v255_unsigned_candidate_build.ps1`. Do not pass ad hoc executable paths to later gates. The build wrapper requires C: strictly above 26.5 GiB and D: strictly above 20.5 GiB before starting, preserves the strict 20 GiB floor while each exact build process tree runs, and emits the authoritative release-artifact report.
8. Install that report's exact NSIS setup into a fresh isolated destination with `run_v255_isolated_install.ps1`. Then run `validation/run_v255_installed_portable_smoke.ps1` with the release-artifact report and install receipt, plus the green consolidated report and its exact Vitest JSON. The smoke derives and rehashes both candidates instead of accepting arbitrary paths. For each candidate it performs a real Calculate → Results → native save → fresh-process reopen journey before its serial breadth crawl. It uses hidden, isolated WebView2/CDP sessions and will only terminate PID trees it created. The portable pass additionally executes the 17 native cross-method routes, including actual Windows effective DPI=192 without changing display settings. Then collect current 2.55 reopen captures, the required post-hoc run, and all specialized/cross-method evidence; build the ZIP, update its curated SHA-256 manifest, and rerun the packaged smoke with `-EvidenceBundlePath` for publication verification.
   The portable pass also runs `v255_frozen_archive_reopen_crawler.mjs` serially: it reopens the 17 reusable archives, stages new current Results captures, and records the one required post-hoc supplement as an explicit block until supplied.
   Without a bundle, the runner checks that the named-evidence index is structurally complete while allowing pending rows. With a bundle, it runs the publication verifier and records the resulting report hash in the installed/portable aggregate receipt.
9. Run the final product-completion audit with the first and final consolidated reports, current publication reports, named-evidence report, and evidence ZIP. It verifies ancestry and permits only version, documentation, curated evidence-index, and generated v2.55 receipt changes after the green gate.
10. Review screenshots, machine-readable observations, artifact hashes, and remaining warnings.
11. Only then merge, tag, and publish the unsigned 2.55.0 release.

## Current Deliberate Gaps

- The 64 setup routes and 18 Calculate captures are declared, but the frozen-result archive index, named-evidence index, and evidence ZIP manifest remain `pending_collection`. They may remain pending during the source gate; every pending Result family or unbound ZIP member blocks publication verification.
- The 14-step consolidated source gate, including its source-level browser crawls, passed in the committed first/final evidence pair. Candidate build, installation, portable/native DPI routes, evidence-bundle collection, and publication have not yet run.
- The release remains unsigned; code signing is outside this program.

## Commands for the Later Gate

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v255_consolidated_diagnostics.ps1

powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v255_unsigned_candidate_build.ps1 `
  -BuildRoot <new-local-build-root> `
  -ArtifactDirectory D:/QuickPLS/target/release/artifacts/<new-v255-artifact-directory> `
  -ReleaseReportPath D:/QuickPLS/target/release/artifacts/<new-v255-artifact-directory>/v255_release_artifacts.json `
  -Label v2_55_0_calculate_evidence

powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v255_isolated_install.ps1 `
  -ReleaseArtifactReportPath D:/QuickPLS/target/release/artifacts/<new-v255-artifact-directory>/v255_release_artifacts.json `
  -InstallRoot <new-isolated-install-directory> `
  -ReceiptPath D:/QuickPLS/target/release/artifacts/<new-v255-artifact-directory>/v255_isolated_install.json

powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v255_installed_portable_smoke.ps1 `
  -ReleaseArtifactReportPath D:/QuickPLS/target/release/artifacts/<new-v255-artifact-directory>/v255_release_artifacts.json `
  -InstallReceiptPath D:/QuickPLS/target/release/artifacts/<new-v255-artifact-directory>/v255_isolated_install.json `
  -VitestReportPath <validation/results/.../v255_full_vitest.json> `
  -ConsolidatedReportPath <validation/results/.../v255_consolidated_diagnostics.json>

# Collect the 55 candidate-bound named observations into a new staging tree.
python validation/v255_named_evidence_collector.py `
  --candidate-report <source-smoke-evidence>/v255_installed_portable_smoke.json `
  --staging-dir <new-named-staging>

# Build the closed deterministic ZIP and three proposed source contracts from
# the named staging plus the portable frozen-archive staging.
python validation/v255_evidence_bundle_builder.py `
  --named-staging <new-named-staging> `
  --named-collector-report <new-named-staging>/named-evidence/provenance/v255-named-evidence-collection.json `
  --named-collected-index <new-named-staging>/v255_named_evidence_index.collected.json `
  --frozen-staging <source-smoke-evidence>/portable/frozen_archive_reopen `
  --frozen-aggregate-report <source-smoke-evidence>/portable/frozen_archive_reopen/receipts/v255-frozen-archive-reopen-crawler.json `
  --output-dir <new-bundle-output>

# Review the builder receipt, then atomically replace the three curated source
# contracts with its proposed named index, frozen index, and bundle manifest.
# Never edit their hashes by hand. Commit only those reviewed contracts before
# the publication-stage smoke below.

# Final publication verification after current 2.55 evidence has been captured
# and its release-attachment ZIP and source manifest have been curated.
powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v255_installed_portable_smoke.ps1 `
  -ReleaseArtifactReportPath D:/QuickPLS/target/release/artifacts/<new-v255-artifact-directory>/v255_release_artifacts.json `
  -InstallReceiptPath D:/QuickPLS/target/release/artifacts/<new-v255-artifact-directory>/v255_isolated_install.json `
  -VitestReportPath <validation/results/.../v255_full_vitest.json> `
  -ConsolidatedReportPath <validation/results/.../v255_consolidated_diagnostics.json> `
  -EvidenceBundlePath <new-bundle-output>/QuickPLS-2.55-evidence.zip

python validation/v255_product_completion_audit.py --publication-stage `
  --expected-product-version 2.55.0 `
  --vitest-report <validation/results/.../v255_full_vitest.json> `
  --rebaseline-report <validation/results/.../interaction_contracts/v255_rebased_interaction_contracts.json> `
  --first-consolidated-report <validation/results/...first.../v255_consolidated_diagnostics.json> `
  --final-consolidated-report <validation/results/...final.../v255_consolidated_diagnostics.json> `
  --method-publication-report <validation/results/.../v255_method_evidence_crawler.json> `
  --installed-portable-report <validation/results/.../v255_installed_portable_smoke.json> `
  --frozen-archive-reopen-report <validation/results/.../receipts/v255-frozen-archive-reopen-crawler.json> `
  --named-evidence-report <validation/results/.../v255_named_evidence_verifier.json> `
  --evidence-bundle <QuickPLS-2.55-evidence.zip> `
  --output <validation/results/.../v255_product_completion_publication_audit.json>
```

The source-stage packaged smoke intentionally refuses to pass until every executable route is present. The collector and bundle builder then fail closed unless all 55 named observations and all frozen Result families are complete. The live Calculate → Results → save → fresh-reopen journey remains mandatory for both candidates even when frozen evidence supplies breadth.
