# QuickPLS v2 Mockup Extra Feature Backlog

This file records existing QuickPLS frontend features that are intentionally hidden, de-emphasized, or deferred after the native mockup shell became the default UI.

Status update through `v2_45_0_mockup_visible_feature_completion`: the default native shell is no longer a static mockup route. v2.43 wired command behavior, v2.44 bound default screens to production workspace state, and v2.45 completed the first visible-feature pass for Data tabs, Model explorer/bottom panes, Setup evidence details, Run output preview, and Trust Center release integrity.

## Deferred Or Hidden During Mockup Parity

- Broad global method catalog in the top bar. The parity route keeps the method selector compact and points detailed choices to Setup.
- Large dashboard-style Home cards and repeated global readiness banners. The parity route uses desktop panels and status chips instead.
- Full v1.x/v1.8 result interpretation lanes on every tab. The parity route keeps the mockup-style right interpretation pane and compact findings.
- Additional report comparison details. The parity route keeps Report focused on the four-step export flow.
- Extended Data workspace helper cards beyond the mockup's quality cards, data tabs, and metadata inspector.
- Dense SEM designer micro-toolbar inside the canvas. The parity route moves primary actions into the ribbon.
- Non-mockup result table controls that duplicate ribbon/menu commands.
- Experimental feature surfacing outside Setup and Trust Center.

These are not deleted. They should be reconsidered only after the default native shell remains stable under real project testing.

## Now Wired In The Default Native Shell

- Project close and unsaved-change guard.
- Data Transform, Add Column, Recode, Missing Values, Filter, and Sort dialogs and command feedback.
- Model ribbon commands for selection, pan, add latent, connect path, covariance, delete, arrange, check diagram, focus diagram, and zoom.
- Results command forwarding for copy/export/workbook/confidence/interpretation/comparison.
- Report export commands for SVG, CSV/HTML table outputs, XLSX workbook, browser Print/PDF path, and Open Folder.
- Trust Center refresh, method documentation, evidence export, and release checksum verification.
- Settings apply, OK, cancel, reset, import/export preferences, and pending-changes behavior.

## Still Intentionally Absent Or Deferred

- True calculation pause/resume. QuickPLS supports cancellation; suspension/resume is not exposed because the current runner is not designed for safe deterministic suspension.
- Native PDF/PNG export. SVG remains the audited publication figure export; browser Print/PDF remains the documented PDF path.
- CPU/memory live telemetry. Fake telemetry was removed; reintroduce only if a lightweight desktop runtime command is implemented.
- SmartPLS project import or equivalence features. These remain unsupported and must not be implied.
- Broad cross-family result comparison beyond currently compatible run comparisons.
- Advanced experimental method controls outside Setup/Trust Center.
