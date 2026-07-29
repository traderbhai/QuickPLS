# QuickPLS v1.6.0 Model Canvas Shell And Panel Polish

## Summary

This milestone closes the model-canvas shell items from the v1.5.7 launch-quality audit. It is a frontend-only change: statistical engines, recipes, result schemas, project format, validation tolerances, and numerical fingerprints are unchanged.

## Completed Scope

- Added a collapsible right inspector so the SEM canvas can use more desktop width.
- Added View menu controls to collapse the left explorer, collapse the right inspector, hide/show the minimap, isolate a selected neighborhood, and collapse indicators for large diagrams.
- Made the minimap opt-in instead of always visible.
- Converted the result overlay banner into compact canvas chrome.
- Reduced selected-object toolbar crowding by grouping secondary indicator, route, and advanced edge actions.
- Simplified explorer construct cards by moving lower-frequency layout and duplicate controls into a More menu.
- Removed remaining model-shell mojibake for `R²`, separators, and path arrows.

## Evidence

- `validation/results/v160_model_canvas_smoke.json`
- `validation/results/v160_model_canvas_audit.json`
- `npm run build`
- `cargo run -p qpls-cli -- gate v1_6_0_model_canvas_shell_and_panel_polish`

## Non-Engine Boundary

This milestone only changes React components, CSS, validation scripts, and documentation for the model canvas shell. It does not change calculation logic or any validated method output.
