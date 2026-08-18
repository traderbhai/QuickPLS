# Method information placement

QuickPLS uses progressive disclosure for method guidance. Static scientific
education has one customer-facing home: **Method Details**.

## Placement rule

| Surface | Show | Do not repeat |
| --- | --- | --- |
| Method picker | Method name and category | Scope, qualification status, assumptions, limitations, or technical design |
| Calculate | Editable settings, selected values, live data profiles, and actionable blockers | Method descriptions, fixed inference contracts, supported-scope paragraphs, or exclusions |
| Specialist setup dialogs | Controls and the next action needed to finish setup | General method education or availability commentary |
| Results | Run-specific findings, diagnostics, and warnings | Generic method scope or setup guidance |
| Run Details and exports | Immutable settings, versions, fingerprints, and provenance | User education already covered by Method Details |
| Method Details | Purpose, applicability, requirements, defaults, outputs, assumptions, limitations, interpretation, technical details, and references | — |

## Active UI inventory

- `NativeCalculationDialog.tsx`: remove repeated catalogue descriptions,
  Experimental/Limited badges and banners, fixed inference summaries,
  supported-scope notes, exclusions, and generic missing-data/execution notes.
  Keep editable controls, dynamic eligibility summaries, complete-case profiles,
  and readiness errors.
- `NativeProcessSetup.tsx`: remove the general PROCESS scope banner. Keep graph
  controls and the live graph assessment.
- `NativeGroupSetupDialog.tsx`: remove the general two-group/MICOM explanation.
  Keep group selection, row profiling, and validation feedback.
- `NativeHigherOrderDialog.tsx`: retain only the immediate next-step instruction;
  move estimator and inference limitations to Method Details.
- `NativeRecipeV4CbsemWorkspace.tsx`: remove static estimator/bootstrap/archive
  contract prose. Keep field-specific missing-data help, preflight diagnostics,
  job status, persistence actions, and failures.
- `NativeModelInspector.tsx`: remove the persistent mode-explanation footer.
  Keep contextual editing help and preflight feedback.
- `NativeResultsSurface.tsx`: preserve run-specific diagnostics and provenance;
  Method Details remains available beside the selected run.

Availability and qualification still control what can be selected. This change
only removes repeated explanatory copy from normal workflow surfaces.
