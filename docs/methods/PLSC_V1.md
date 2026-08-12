# PLSc v1 legacy compatibility note

Status: legacy and non-current. QuickPLS no longer emits `plsc_v1` results.

PLSc v1 used a provisional reliability expression in its attenuation correction while labeling the payload's `reliability_method_version` as `dijkstra_henseler_rho_a_v1`. That expression was not Dijkstra and Henseler's Equation 3. Its Python reference repeated the same provisional expression, so agreement with that reference did not independently establish the rho_A claim.

Existing project archives remain readable only when all of these values agree exactly:

- the analysis method is `plsc`;
- the estimation payload and nested PLSc payload both use `plsc_v1`;
- immutable provenance contains `plsc_v1`;
- the payload retains the historical reliability-version field.

Loading such a result adds the `plsc.legacy_method_version` warning. The stored numbers are preserved for auditability and are not silently rewritten, reinterpreted, or promoted to the current method. Re-run the analysis to obtain the current `plsc_v2` correction described in `PLSC_V2.md`.

PLSc v1 must not be cited as a validated implementation of Dijkstra-Henseler rho_A. The legacy compatibility path exists solely to avoid making prior QuickPLS projects unloadable.
