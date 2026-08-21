# QuickPLS 2.54.0 — Canvas and Results Refinement

Status: **source implementation and version-authority coordination are complete.
The unsigned Windows candidate, release-artifact package, packaged lifecycle
smoke, and any GitHub publication are still pending.**

## What changed

QuickPLS 2.54 refines the existing `Canvas → Calculate → Results` workflow. It
does not add a calculation method, change a numerical engine, broaden an
estimand, or relabel an existing scientific or archive identity.

### Canvas editing and navigation

- Visible model edits use one authority-aware command path with explicit
  applied or blocked outcomes. Scientific changes retain stable identities,
  undo semantics, and the existing strict revision authority; presentation
  changes remain presentation-only.
- Indicator, construct, path, HOC, and moderation actions share the same edit
  contract instead of silently behaving differently between Canvas,
  Properties, and menu entry points.
- The model navigator now separates Indicators, Constructs, and Relationships.
  Relationship rows use authored names and suppress generated technical rows.
- Canvas arrangement adds tidy-selection, alignment, distribution, and
  left-to-right or top-to-bottom model strategies. Fit offers structure, all,
  and selection scopes.
- Indicator side and saved manual-placement controls, pins, route geometry, and
  label offsets survive unrelated model mutations and arrangement actions.

### Researcher-facing Results

- Normal Results resolve construct, indicator, path, mediation, moderation, and
  higher-order identities from the immutable model snapshot. Generated IDs and
  hashes remain available only where diagnostic detail is appropriate.
- Result tables identify the primary label column, keep it sticky where
  applicable, align scientific values numerically, retain horizontal overflow,
  and expose the relevant confidence level.
- Higher-order method details are grouped with Higher-order Constructs, and
  structural-randomization exports use the same authored path labels as the
  visible Results table.
- Empty Results provides one compact **Calculate results** action that opens the
  unchanged 18-method catalogue.

## Verification status

The latest consolidated source diagnostic recorded **8 of 9 steps passing**.
Rust authority, archive, and routing checks, Rust compilation, frontend
typecheck, production frontend build, diff check, and the headless
Canvas/Results crawl passed. The crawl confirmed the 20-construct/80-indicator
fixture, three-part model navigator, inspector handoff, appearance controls,
Arrange menu, Results empty state, unchanged 18-method catalogue, and no browser
console errors.

The remaining consolidated failure was the full frontend Vitest step. After the
failure set was reviewed and corrected, the focused remediation run passed
**69 of 69 targeted tests**. A later complete nine-step rerun has not been
claimed.

## Remaining release work

Before 2.54 can be described as a packaged Windows release:

1. Build one frozen unsigned Windows candidate.
2. Package setup, portable, CLI, and checksum artifacts with the
   `v2_54_0_canvas_results` label.
3. Run the automated packaged Canvas → Calculate → Results → save → fresh-reopen
   journey against that same candidate.
4. Record the resulting screenshots and observations.
5. Publish a GitHub prerelease only after those steps succeed.

The latest published public prerelease therefore remains 2.53.0 until this
pending release work is completed.
