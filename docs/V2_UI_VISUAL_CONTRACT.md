# QuickPLS 2.0 Visual Contract

QuickPLS 2.0 uses the selected desktop mockup as the visual contract for all workspace rebuilds. This contract is intentionally product/UI-only. It does not change statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## Contract Status

- Source of truth: the approved QuickPLS 2.0 desktop mockup plus this document.
- Implementation rule: every new v2 workspace must compose shared `qpls2` primitives before adding screen-specific classes.
- QA rule: a screen is incomplete until it has static audit evidence, desktop visual evidence at the required viewports, and the v2 visual gap audit has not found a high-severity rendered-screen issue.
- Release rule: every v2 milestone must produce a versioned installer, portable executable, and checksum file under `D:\QuickPLS\target\release\artifacts`.

## Desktop Target

- Primary viewport: `1440x900`.
- Secondary viewport: `1280x800`.
- Windows desktop is the target. Mobile behavior is non-gating.
- The app should feel like a dense professional research workstation, not a marketing site or prototype dashboard.

## Shell Structure

- Top title bar: app identity, project name, milestone label.
- Command bar: file commands, compact method selector, scoped status chip, Run button, nearby blocker reason.
- Workflow strip: Data, Model, Setup, Run, Results, Report.
- Support utility shell: Home, Trust Center, and Settings use local support navigation instead of workflow progress controls.
- Left rail: persistent workspace navigation.
- Workspace content: centered maximum width, consistent gutters, low-radius panels, dense but readable controls.
- Status bar: readiness chips, dataset/model counts, save/autosave state, offline state, validated scope.
- Home, Trust Center, and Settings are launcher/support utilities. They must not show the primary calculation workflow strip or coach, but they should provide local switching among launcher, evidence/scope, and preferences.
- Data, Model, Setup, Run, Results, and Report are the calculation workflow. Model may keep a dedicated SEM Designer workflow band instead of the generic page-host workflow treatment.
- Support utility controls should align to the same workspace gutters as their page content and must not introduce horizontal overflow at `1440x900` or `1280x800`.

## Mockup-Matching Rules

- First viewport must show the workspace title, primary action, and the first decision surface without forcing vertical scrolling.
- Workspace content should align to the same left edge across Home, Data, Setup, Run, Results, Report, Trust Center, and Settings.
- Command bars and workflow strips must stay calm: no dense warning paragraphs, no duplicate primary actions, and no mixed button heights.
- Cards are used for individual decisions or metrics only; large page sections should be panels or split workbench areas.
- Tables, canvases, and preview panes may scroll internally; the full app page should not create avoidable horizontal scrolling.
- Empty states must tell the user the exact next action and where it happens.
- The active state must be visible but restrained; teal is reserved for active navigation, validated scope, and primary actions.
- Warning surfaces must be local to the action or section they affect.

## Design Tokens

The v2 token family lives in `src/styles.css` under `--q2-*`.

- Background: `--q2-bg`.
- Panel: `--q2-panel`.
- Muted panel: `--q2-panel-muted`.
- Border: `--q2-border`.
- Strong border: `--q2-border-strong`.
- Primary text: `--q2-text`.
- Heading ink: `--q2-ink`.
- Muted text: `--q2-muted`.
- Accent: `--q2-teal`.
- Accent dark: `--q2-teal-dark`.
- Accent soft: `--q2-teal-soft`.
- Success/warning/danger: `--q2-success`, `--q2-warning`, `--q2-danger`.
- Radius: `--q2-radius`.
- Page gutters: `--q2-page-gutter-x`, `--q2-page-gutter-y`.
- Panel padding: `--q2-panel-pad`.
- Section gap: `--q2-section-gap`.
- Control height: `--q2-control-height`.
- Toolbar height: `--q2-toolbar-height`.
- Shadow: `--q2-shadow-soft`.

## Component Rules

Every redesigned screen should use the shared v2 primitives where possible:

- `.qpls2-workspace`
- `.qpls2-panel`
- `.qpls2-panel-title`
- `.qpls2-page-title`
- `.qpls2-page-subtitle`
- `.qpls2-command-row`
- `.qpls2-card-title`
- `.qpls2-card-body`
- `.qpls2-chip`
- `.qpls2-primary-action`
- `.qpls2-secondary-action`

Screen-specific classes may exist, but they should compose these primitives instead of inventing separate visual systems.

## Layout Rules

- Avoid horizontal page scrolling except inside data/result tables and canvases.
- Keep the first viewport useful at `1440x900`: the workspace header and the primary action should be visible.
- Prefer split workbench layouts over large sparse cards.
- Use restrained borders and shallow shadows.
- Cards must not place title and body text on the same visual line.
- Disabled primary actions must have a visible nearby reason.
- Dense data surfaces should use sticky headers, clear overflow hints, and compact controls.

## Typography Rules

- Use `Inter`, `Segoe UI`, `Arial`, sans-serif.
- Letter spacing is `0` except uppercase eyebrow labels.
- Page titles use strong but restrained desktop type.
- Table text remains compact.
- Avoid large hero-style type inside operational screens.

## Status And Claims

- Use `Validated scope`, `Experimental`, `Unsupported`, and `Needs setup` consistently.
- Do not claim SmartPLS equivalence.
- Do not claim unsupported or unaudited method shapes are publication-ready.
- Interpretation copy must remain conservative and value-specific.

## SEM Designer Boundary

The SEM canvas grammar remains the stable academic diagram style: latent ovals, indicator rectangles, straight structural arrows, measurement arrows, R²/loadings/path overlays after compatible runs, and SVG parity. v2 work may redesign the surrounding shell, toolbar, explorer, and inspector, but should not destabilize validated diagram behavior.

## Screen Completion Checklist

Every redesigned screen must prove:

- Shared `qpls2` primitives are present.
- The screen title and main action are visible at `1440x900` and `1280x800`.
- Primary disabled actions have a nearby reason.
- Table or canvas overflow is intentional and labeled.
- No visible text collision is possible from title/body inline concatenation.
- Status copy uses `Validated scope`, `Experimental`, `Unsupported`, or `Needs setup`.
- No stale version label, R-squared mojibake, or SmartPLS-equivalence claim appears.
- Smoke/audit output is written under `validation/results/`.

## Acceptance

A v2 screen is not considered complete until:

- It uses the shared v2 token/component family.
- It matches the selected mockup direction at `1440x900` and `1280x800`.
- It has no visible text collisions, mojibake, stale version text, or unsupported claims.
- Primary actions and disabled reasons are clear.
- Visual smoke/audit evidence exists under `validation/results/`.
- Existing numerical tests and gates remain clear.
