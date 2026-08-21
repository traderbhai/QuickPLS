# QuickPLS 2.51.0 — Unified Calculation Workflow

Status: **verified unsigned preview candidate**. The unified workflow, consolidated automated checks, Windows build, packaged smoke journey, screenshots, and checksums are complete on this branch. Code signing and GitHub publication remain pending.

## One model workflow

Version 2.51 uses one researcher-facing journey:

1. Import and inspect data.
2. Build or revise the model on Canvas.
3. Choose `Calculate`.
4. Select and configure one of the existing 18 methods.
5. Monitor or cancel the calculation.
6. Review the automatically opened categorized Results.
7. Export, save, close, and reopen the same result.

Canvas is the only permanent model-authoring document. Parameter Table, General SEM, and Exact CB-SEM are no longer separate permanent tabs.

## Feature-aware calculation

The Calculate catalogue remains at 18 methods. QuickPLS detects the resident model features and routes the selected method internally:

- **PLS Algorithm** handles eligible point estimation, including detected mediation, simultaneous moderation, and higher-order output.
- **Bootstrapping** handles eligible multiple mediation, simultaneous moderation, higher-order, and bounded first- or second-stage moderated mediation.
- **CB-SEM** handles eligible common-factor ML and exposes case bootstrap as a method option when the model supports it.

Setup shows detected features, applicable settings, expected result groups, and corrective actions. A blocked setup does not modify the diagram.

This release changes orchestration and presentation. It does not rename the qualified Rank 0–3 cells, expand their scientific predicates, or reinterpret historical results.

## Advanced model editing

- Moderating effects and higher-order constructs remain Canvas commands.
- Mediation is detected from drawn structural paths.
- Bounded moderated mediation is selected while configuring PLS Bootstrapping.
- The resizable **Advanced Parameter Table** opens from Canvas, CB-SEM setup, or a corrective preflight action.
- Parameter-level changes and older projects use a safe calculation-ready revision; the source project is not silently overwritten.

Historical General SEM and Exact CB-SEM projects remain readable through internal compatibility adapters. Those adapters do not appear as separate workspace tabs.

## Results and export

Successful calculations open the normal Results workspace. The sidebar shows only categories owned by the completed run, including applicable measurement, structural, direct/indirect/total effect, moderation, higher-order, moderated-mediation, CB-SEM parameter, fit, bootstrap, diagnostic, and run-detail groups.

Canonical result values remain the authority for display, persistence, and supported CSV, XLSX, self-contained HTML, PDF, SVG, and PNG exports. Existing method and archive identities remain unchanged.

## Compatibility and limits

- Existing Version 2.50 projects and results retain their recorded identities.
- Scoped Standard continues to mean only the exact documented predicate; it does not claim unrestricted SmartPLS parity or identity with undocumented behavior.
- No SmartPLS project import is added.
- Experimental Labs remains separate and disabled by default.
- The Windows candidate remains unsigned unless a later signed release gate is completed.

## Verification and preview artifacts

The code-first consolidated pass completed successfully:

- Rust formatting and the focused archive workflow suite passed (21 tests).
- Full frontend typechecking passed.
- Focused unified-workflow tests passed (108 tests).
- The production frontend bundle and locked release desktop/CLI builds completed.
- The packaged Tauri journey passed with no console errors, covering Canvas, the 18-method Calculate dialog, corrective CB-SEM guidance, categorized Results, live progress, and cancellation.
- The packaged test app exited cleanly with no remaining QuickPLS process or test ports.

Packaged journey evidence is recorded in [the smoke report](../validation/results/v251_unified_workflow_packaged_smoke.json). Its screenshots and observations are:

| State | Screenshot | Observation |
|---|---|---|
| Canvas | [01-canvas.png](../validation/results/screens/v251-unified-workflow/01-canvas.png) | Canvas is the single permanent model workspace and exposes contextual advanced actions. |
| Method setup | [02-method-setup.png](../validation/results/screens/v251-unified-workflow/02-method-setup.png) | Calculate contains exactly 18 methods and method-specific settings. |
| Corrective error | [03-corrective-error.png](../validation/results/screens/v251-unified-workflow/03-corrective-error.png) | Ineligible CB-SEM remains in Calculate and explains how to correct the setup without changing the model. |
| Results | [04-results.png](../validation/results/screens/v251-unified-workflow/04-results.png) | A completed model opens the normal categorized Results workspace. |
| Progress | [05-progress.png](../validation/results/screens/v251-unified-workflow/05-progress.png) | A packaged calculation exposes live progress and cancellation in the Calculate window. |

The preserved unsigned-preview files are listed in [release_artifacts.json](../validation/results/release_artifacts.json). Primary SHA-256 values are:

- Setup: `D8684050E2BC1532DD08DAA1183F2917E386B5E054F914C1F2EC582EBBD3EEC8`
- Portable desktop: `31A519B625B242D0175A88746CB27CA4C43FF648DAD0639960FB5364869C04DC`
- CLI: `E441C6185245228D81D595F267657731BC33F24FC651989FD394C91A0CED75E2`

This candidate is not yet signed or published. Checksums verify file identity; they do not establish publisher authenticity.

See also:

- [Quick Start](QUICK_START.md)
- [User Guide](USER_GUIDE.md)
- [Method Compatibility](METHOD_COMPATIBILITY.md)
- [Known Issues](KNOWN_ISSUES.md)
- [Version 2.50 SEM Upgrade Notes](RELEASE_NOTES_V2_50_0.md)
