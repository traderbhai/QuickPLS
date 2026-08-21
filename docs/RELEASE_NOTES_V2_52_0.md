# QuickPLS 2.52.0 — Native HOC and Model-Fit Refinement

Status: **implemented and verified as an unsigned Windows candidate**. QuickPLS 2.52.0 has not been code-signed or published from this workspace.

## Diagram-native higher-order constructs

Higher-order constructs (HOCs) stay inside the normal Canvas → Calculate → Results workflow without a new tab, ribbon, or permanent instruction panel.

- Select at least two eligible constructs and open **Higher-Order Construct…** from the Model menu or selection context menu.
- Choose whether the HOC explains its dimensions or the dimensions form the HOC. QuickPLS derives RR/RF/FR/FF from that direction and the dimensions' existing Mode A/B measurement.
- The dialog recommends disjoint two-stage for eligible measurement-only dimensions and embedded two-stage when eligible dimensions already participate in structural relationships.
- Repeated and extended-repeated approaches appear under **Advanced** only for supported topology/type combinations. Hybrid remains read-only compatibility.
- Edit a selected HOC from its context menu, the Model menu, Properties, or Enter. Activated immutable projects use one atomic **Save As Revision** transaction while preserving the original archive and existing scientific identities.
- The diagram shows only the normal HOC label and compact marker. Selecting it highlights its dimensions with presentation-only membership connectors that are not persisted as scientific edges.

Nested or multiple HOCs and HOC interactions remain outside the current bounded scope.

## Compact calculation and results

The Calculate catalogue remains at exactly 18 methods. An eligible HOC appears as one compact name/type/approach row with an **Edit…** action.

- **PLS Algorithm** routes an eligible HOC to its existing point cell.
- **PLS Bootstrapping** routes it to the existing point plus full-model case-bootstrap cells.
- Unsupported combinations show one concise cause and corrective action.

Results use researcher-facing groups for component relationships, HOC structural paths, extended effects when applicable, and bootstrap inference. Internal receipts, hashes, and raw identities remain under Run Details. HOC workflows continue to omit inapplicable model-fit tables and state only that model fit is not reported for that workflow.

## Descriptive PLS model fit

The ordinary PLS fit entry is renamed **Model fit — descriptive**. Its interpretation moves from a full-width amber warning to a neutral information button and Model Fit Details.

Properties shows one payload-derived state:

- **Exact-fit bootstrap: Not run**;
- **Exact-fit results available**;
- **Exact-fit results partial**;
- **Exact-fit results unavailable**; or
- **Exact-fit run failed**.

Amber or red is reserved for exact-fit inference that was requested but incomplete or failed. The adapted Bollen–Stine Registry cell remains unqualified and hidden from Calculate in 2.52. Historical reason codes and result identities are preserved, and the full scientific explanation remains in Model Fit Details and exports.

## Compatibility

This release changes authoring and presentation, not estimators, estimands, Registry cells, bootstrap behavior, or schema-6 result identities. Existing HOC and model-fit results remain readable under their recorded identities.

## Verification and candidate

The consolidated Rust/frontend diagnostic pass, focused HOC and archive tests, TypeScript build check, production frontend build, and final Windows build completed successfully. The final automated packaged journey created and edited an HOC, routed PLS calculation, opened categorized Results, saved the schema-6 archive, restarted QuickPLS, and verified the same result after reopen.

- Portable executable: `QuickPLS_2.52.0_unsigned-preview_v2_52_0_native_hoc_fit_20260821-120805_x64_portable.exe`
- Setup executable: `QuickPLS_2.52.0_unsigned-preview_v2_52_0_native_hoc_fit_20260821-120805_x64_setup.exe`
- CLI executable: `QuickPLS_2.52.0_unsigned-preview_v2_52_0_native_hoc_fit_20260821-120805_x64_cli.exe`
- Checksums: `QuickPLS_2.52.0_unsigned-preview_v2_52_0_native_hoc_fit_20260821-120805_x64_checksums.txt`
- Packaged smoke report: `validation/results/v252_hoc_packaged_smoke_20260821_120805_r2/v252_hoc_packaged_smoke.json`
- Screenshots: six PNG files under the report's `screens/` directory.

The portable executable SHA-256 is `E7D521F1BA247911B3D5EE96A581B490978D4A26E91CCD67BDED8886E326A2A6`. Authenticode verification was intentionally not performed. Free space remained above the 20 GiB floor after the build and smoke (C: 28.36 GiB; D: 33.21 GiB).

See also:

- [User Guide](USER_GUIDE.md)
- [Quick Start](QUICK_START.md)
- [General SEM PLS higher-order constructs v1](GENERAL_SEM_PLS_HIGHER_ORDER_V1.md)
- [PLS Model Fit v2](methods/PLS_MODEL_FIT_V2.md)
- [Version 2.51 Release Notes](RELEASE_NOTES_V2_51_0.md)
