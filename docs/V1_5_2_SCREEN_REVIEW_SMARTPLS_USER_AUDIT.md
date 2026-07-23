# QuickPLS v1.5.2 Screen Review: SmartPLS-User UX Audit

Date: 2026-07-23
Scope: Review of user-provided desktop screenshots for Home, Data, Model, Setup, Run, Results, and Report.
Perspective: Academic researcher familiar with SmartPLS-style SEM workflows.

This audit records design observations only. It does not change statistical engines, method scopes, formulas, validation tolerances, project schemas, or result payloads.

## Executive Assessment

QuickPLS has moved into a coherent desktop workflow, especially after the navigation, SEM designer, toolbar, sidebar, and Data workspace hardening work. The remaining UX gap is no longer the overall product direction. The gap is presentation precision: text collisions, excessive card spacing, repeated warnings, scroll-state issues, and uneven control hierarchy make the app feel less mature than SmartPLS even when the underlying workflow is now strong.

Current UX health estimate: 6.5/10.

Most important next move: a focused v1.5.3 polish pass on typography, spacing, disabled-state reasons, screen scroll behavior, and Setup/Report clarity.

## Cross-Screen Findings

### P0: Text Collision And Inline Copy Bugs

Several screens show heading and body text rendered without spacing:

- `Start new projectStart from...`
- `Import datasetCSV...`
- `Open demo projectUse...`
- `1. DataCorporate Reputation.csv...`
- `Missing dataset9 variables loaded`
- `Experimental scopeValidated for...`
- `Diagram exportSVG is...`
- `Table exportsRun a method...`

This is the highest-priority visual defect because it immediately signals unfinished UI. It should be fixed before deeper redesign work.

Recommended fix:
- Make reusable card header/body components enforce block layout.
- Add visual regression screenshots for cards with long labels.
- Add a static audit for headings followed directly by body text without spacing.

### P0: Repeated Global Warning Dominates Every Screen

The top orange warning about dataset fingerprint appears across all screens and competes with primary actions. The same issue also appears in Setup, Run, Results, and Report, creating repetition.

Recommended fix:
- Replace the large persistent banner with a compact clickable status chip: `1 blocker: dataset not imported`.
- Show the full explanation only where it is actionable: Data, Setup, and Run.
- Make the Run button's disabled reason appear directly beside the Run button.

### P1: Screens Open Or Remain At Mid-Scroll

Several screenshots show Setup and Report in mid-page scroll states. This makes navigation feel unstable.

Recommended fix:
- Reset scroll to top on rail navigation.
- Preserve scroll only when returning to the same workspace intentionally.
- Add a smoke test that clicks every rail item and verifies the page starts at the expected header.

### P1: Card-Heavy Layout Feels Sparse For Desktop Research Software

SmartPLS-style tools are dense, table-oriented, and workflow-first. QuickPLS currently uses many large cards, which creates large blank areas and long scrolling.

Recommended fix:
- Use compact cards only for readiness and warnings.
- Prefer split panels, tables, sticky headers, and tool strips for workspaces.
- Reduce vertical padding across Data, Setup, Run, Results, and Report.

### P1: Disabled Actions Need Nearby, Specific Reasons

Disabled buttons are present, but the reason is sometimes far away or duplicated elsewhere.

Recommended fix:
- Every disabled primary action should include a short inline reason under or beside the button.
- Use action-specific copy, not generic readiness copy.
- Example: `Run disabled: import the dataset into the desktop project first.`

## Screen-Specific Review

## 1. Home

What works:
- The left rail workflow is clear.
- The current project name is visible.
- The page explains the general project flow.

Problems:
- Multiple card headings collide with body text.
- The screen feels like a collection of cards rather than a professional desktop start hub.
- `desktop-first workflow` floats without adding much value.
- `Current workspace not saved yet` should offer a direct `Save project` action, not mainly `Open project`.

Recommended changes:
- Fix card typography immediately.
- Convert Home into a tighter project launcher: Recent projects, Open, Import dataset, Demo, Recovery.
- Move version/status language away from the main content.

## 2. Data Workspace

What works:
- The new structure is much clearer.
- Data Quality cards are useful.
- Prefix construct creation is a strong bridge into model design.
- Metadata editing is visible and understandable.

Problems:
- The page still consumes too much vertical space before the data grid.
- Data grid has a large empty area for small datasets.
- The selected-column metadata editor is useful but should include descriptive statistics.
- `Validation details` is still a developer-flavored phrase.
- The raw import copy and controls feel slightly cramped.

Recommended changes:
- Make the data grid the dominant area.
- Add per-column stats: missing count, mean, standard deviation, min, max, unique values.
- Rename `Validation details` to `Sample dataset details` or hide it unless developer mode is active.
- Add a column profile sidebar for selected variables.

## 3. Model Designer

What works:
- The SEM diagram is now much closer to SmartPLS/AMOS grammar.
- The left SEM Explorer is stronger than before.
- Context actions are closer to normal researcher workflows.

Problems:
- The contextual toolbar is still crowded.
- Many small icon buttons in construct cards are hard to understand.
- Generic `Path` labels clutter edit mode.
- Construct labels are too small.
- Some path labels collide with arrows or constructs.
- The right inspector still consumes significant space.

Recommended changes:
- Replace dense context toolbar text buttons with grouped menus and tooltips.
- Hide generic `Path` labels by default in edit mode; show only on hover/selection.
- Improve label collision avoidance and label font sizing.
- Add a collapsible right inspector mode.
- Make selected edge and selected edge label controls visually stronger.

## 4. Setup

What works:
- Setup is now correctly separated from Run.
- Readiness cards are useful.
- Basic/Expert separation is the right direction.
- Method presets are useful in concept.

Problems:
- Severe text collisions in readiness/status cards.
- `Experimental scope` contradicts the validated method wording.
- Method presets look like empty boxes and do not feel clickable.
- Basic setup controls are spread too far apart.
- Bootstrap checkbox is visually oversized and detached from its label.
- Group/prediction workflows are visible even when not needed.

Recommended changes:
- Rename `Experimental scope` to `Scope status`.
- Make preset cards compact, selectable, and visibly active/inactive.
- Use proper checkbox/toggle layout for Bootstrap.
- Move group/prediction workflows under Expert or method-specific sections.
- Keep the selected method setup visible near the top without requiring long scroll.

## 5. Run

What works:
- The screen purpose is clear.
- Readiness summary before running is helpful.
- The after-completion explanation is useful.

Problems:
- The disabled Run button reason is too far away.
- The page is sparse and duplicates much of Setup.
- `Open results` appears even when no result exists.
- Disabled button text has low contrast.

Recommended changes:
- Put the disabled reason directly under `Run selected method`.
- Make `Open results` disabled with a reason until a result exists.
- Collapse readiness cards into a compact checklist here.
- Focus Run on execution, progress, cancellation, and handoff.

## 6. Results

What works:
- The empty state is calm and understandable.
- It correctly avoids pretending results exist before a run.

Problems:
- The primary next action should be `Open Data` or `Import dataset`, because Data is the blocker.
- The screen has too much blank area.
- It does not preview the structure of future results.

Recommended changes:
- Use the exact blocker as the primary CTA.
- Show disabled result tabs or a lightweight preview of Summary, Measurement, Structural, and Inference sections.
- Add a `Run method` CTA only when readiness is clear.

## 7. Report

What works:
- The report workflow has the right ingredients: presets, diagram preview, table exports, provenance, and SVG.
- The SmartPLS-like diagram preview is visually close to academic SEM output.

Problems:
- Severe text collisions in export cards.
- Checkboxes are oversized and labels are separated.
- Export presets look like unrelated plain buttons.
- Preview is clipped and requires too much scrolling.
- Disabled export tiles do not explain enough locally.

Recommended changes:
- Convert export presets into selectable cards or segmented controls.
- Fix checkbox sizing and label alignment.
- Fit the diagram preview into the available viewport.
- Add inline disabled reasons to CSV, HTML, XLSX, SVG, and PDF options.
- Keep SVG as the primary export and visually distinguish it from post-v1 native PDF/PNG.

## Accessibility Risks

- Text collisions make screen reader and visual comprehension harder.
- Low-contrast disabled buttons may fail contrast expectations.
- Icon-only controls in SEM Explorer likely need stronger accessible names and visible tooltips.
- Oversized checkboxes with separated labels are harder to parse.
- Mid-scroll navigation can disorient keyboard users.

## Recommended v1.5.3 Milestone

Name: `v1_5_3_layout_copy_readiness_polish`

Scope:
- Fix all heading/body text collisions.
- Replace repeated global warnings with compact blocker chip plus contextual explanations.
- Reset scroll position on workspace navigation.
- Rename confusing labels such as `Experimental scope`.
- Tighten Setup, Run, Results, and Report vertical density.
- Improve disabled-action reasons.
- Improve report checkbox and export-card layout.
- Add visual smoke screenshots for every primary rail workspace at 1440x900.

Acceptance:
- No visible text collisions.
- No normal user-facing `Validation fixture` wording.
- Disabled primary actions always have nearby reasons.
- Every rail screen opens at a predictable top position.
- Report controls look aligned and publication-ready.
- Setup labels do not contradict method validation status.

