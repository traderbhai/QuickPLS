"""Preserve user-supplied launch-quality screenshots and write an issue register.

This milestone is intentionally an audit/planning gate, not a UI rewrite. It
turns the supplied screen evidence into a durable, testable backlog so later
implementation slices can be prioritized without losing the visual context.
"""

from __future__ import annotations

import json
import shutil
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
SCREEN_DIR = RESULTS / "screens" / "v157" / "ui-ux-launch-quality"
OUTPUT = RESULTS / "v157_ui_ux_launch_quality_smoke.json"

SCREENSHOTS = [
    ("01_home.png", "Home", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-7f2c234b-f4a0-49e9-9a2e-ad96aaad85ac.png"),
    ("02_data_top.png", "Data", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-14acc694-53bc-4fee-81c8-2240802b11bc.png"),
    ("03_data_preview_top.png", "Data", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-8ce2680a-06d8-470f-82e6-696cb1aee30d.png"),
    ("04_data_metadata.png", "Data", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-dea223e5-3f11-454e-9c42-e5ab6451b869.png"),
    ("05_model_canvas.png", "Model", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-3443911b-4541-461a-bf6a-f129b0d2941f.png"),
    ("06_setup_top.png", "Setup", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-533e1a50-c678-4600-bc56-d02e606f25ca.png"),
    ("07_setup_presets.png", "Setup", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-d0f1a122-adda-485b-9daa-c1362400c7fd.png"),
    ("08_setup_method_table.png", "Setup", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-494592e2-7e14-47a5-bef9-7187433dcca0.png"),
    ("09_run.png", "Run", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-55acddcf-ab02-44a9-9146-932265c492dd.png"),
    ("10_results_overview_top.png", "Results", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-af91b83a-d3be-48ac-99d9-35d8893055ce.png"),
    ("11_results_overview_table.png", "Results", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-13b6b934-7ec4-4103-b79e-1cd2625cee6d.png"),
    ("12_results_structural_tables.png", "Results", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-08b0b199-c43a-4027-a65b-581854684ce7.png"),
    ("13_report_setup.png", "Report", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-b6b8a23d-bf5f-4c02-96d5-64190f333f46.png"),
    ("14_report_diagram_preview.png", "Report", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-eff34917-99d1-4e51-9134-7a846b9de6ba.png"),
    ("15_report_exports_comparison.png", "Report", r"C:\Users\MOHD~1.NAV\AppData\Local\Temp\codex-clipboard-bfbae2ad-a708-4706-97bd-41510d1c912c.png"),
]

ISSUES = [
    ("UX-001", "high", "global", "Global", "Version/status mismatch", "Header evidence shows stale v1.5.3 text while package metadata has advanced beyond it.", "Wire visible version/status from package/app metadata and add static drift check."),
    ("UX-002", "high", "global", "Global", "Primary run action appears on every workspace", "The top Run button competes with Data, Model, Results, and Report tasks.", "Make the global run action compact outside Setup/Run or make it contextual."),
    ("UX-003", "medium", "global", "Global", "Teal carries too many meanings", "Brand, active navigation, success, selected cards, and primary actions all use similar teal.", "Separate brand/action/status palettes and reserve success green for validation."),
    ("UX-004", "medium", "global", "Global", "Bottom status bar is noisy", "Many chips and messages compete with the main workflow.", "Group status by project/data/model/method and collapse low-priority chips."),
    ("UX-005", "medium", "global", "Global", "Nested scrollbars feel heavy", "Data, Results, and Report show page scroll plus component/table scrolls.", "Define a scroll ownership rule for each workspace."),
    ("UX-006", "medium", "global", "Global", "Cards have weak hierarchy", "Most cards share the same border/fill/weight regardless of importance.", "Create primary, secondary, warning, and table-section card variants."),
    ("UX-007", "medium", "global", "Global", "Desktop density is inconsistent", "Some areas are sparse while tables and toolbars are crowded.", "Standardize page gutters, section spacing, and row density tokens."),
    ("UX-008", "medium", "global", "Global", "Stepper and rail duplicate progress", "The workflow strip repeats the left rail and sometimes does not add actionable state.", "Convert strip into a compact project-readiness ribbon or remove on dense workspaces."),
    ("UX-009", "low", "global", "Global", "Icon style is not fully unified", "Some icons feel heavier or less aligned than adjacent labels.", "Audit icon size/stroke and align all icon buttons to one token."),
    ("UX-010", "medium", "global", "Global", "Accessibility cannot be inferred from screenshots", "Keyboard/focus/contrast states are not visually documented for launch QA.", "Add a launch accessibility smoke covering focus order, table navigation, and contrast."),
    ("UX-011", "medium", "layout", "Home", "Home feels static and underpowered", "The current launcher looks clean but does not feel like a mature recent-project hub.", "Add true recent projects, recovery status, and last-run summary."),
    ("UX-012", "low", "copy", "Home", "Home workflow cards duplicate navigation", "Data/Model/Run/Report cards repeat rail steps without much extra context.", "Make cards contextual to the current project state or remove redundant ones."),
    ("UX-013", "medium", "layout", "Home", "Large unused middle/right area", "The screen has substantial empty space at desktop size.", "Use a two-column launcher: primary actions left, project/recent/evidence right."),
    ("UX-014", "medium", "workflow", "Home", "Continue recent project is not genuinely recent", "The card says use Open rather than listing actual projects.", "Replace with real recent project cards or hide when unavailable."),
    ("UX-015", "low", "copy", "Home", "Current workspace save message is understated", "Autosave/recovery is important but the card reads like a passive notice.", "Use a clearer project state panel with Save as primary when unsaved."),
    ("UX-016", "high", "layout", "Data", "Import source is visually larger than the data itself", "The import area dominates even after data exists.", "Collapse import source after successful import and make the grid dominant."),
    ("UX-017", "medium", "tables", "Data", "Data grid lacks mature desktop controls", "No visible column resize, sorting, freeze options, or compact mode controls.", "Add resize/sort/freeze/density controls and stronger selected-column state."),
    ("UX-018", "medium", "workflow", "Data", "Data quality cards are not actionable enough", "Warnings do not clearly lead to Setup/Run consequences.", "Add action links from each card to metadata, setup, or issue detail."),
    ("UX-019", "medium", "copy", "Data", "Prefix chips look technical", "COMP -> 3 indicators is useful but visually raw.", "Show construct-name preview with indicator count and confidence."),
    ("UX-020", "medium", "layout", "Data", "Metadata panel can overtake inspection work", "Right panel consumes width even when the user only wants data review.", "Allow collapsing metadata and keep column profile as a drawer."),
    ("UX-021", "low", "copy", "Data", "Browser/desktop import copy is too implementation-oriented", "Researchers should not need to think about browser preview versus desktop commands.", "Use user-centered copy focused on file formats and saved project state."),
    ("UX-022", "medium", "layout", "Model", "Context toolbar is crowded", "Selected construct toolbar exposes too many small actions in one line.", "Keep Rename/Delete/Tidy visible and move side/pin/advanced actions into menus."),
    ("UX-023", "high", "diagram", "Model", "Diagram labels remain fragile", "Some path/loading labels are tiny and can collide with lines or constructs.", "Add label collision rules and bigger readable label tokens."),
    ("UX-024", "medium", "layout", "Model", "Left explorer construct cards are too dense", "Cards combine counts, icons, side controls, and status in a compact block.", "Use row-based construct list with hover/context actions."),
    ("UX-025", "medium", "workflow", "Model", "Right inspector duplicates canvas/sidebar controls", "Inspector repeats actions already visible in context toolbar and explorer.", "Make inspector primarily properties/results; move common actions to context menus."),
    ("UX-026", "medium", "diagram", "Model", "Mini-map feels decorative", "It takes space but gives little readable navigation value.", "Replace with useful overview/focus controls or hide by default."),
    ("UX-027", "medium", "layout", "Model", "Canvas width is constrained by two side panels", "SEM diagram is central but gets less width than needed.", "Add one-click collapse and remember panel state."),
    ("UX-028", "medium", "copy", "Model", "Result overlay banner is redundant after visible values", "The banner consumes canvas space while the diagram already shows overlays.", "Replace with compact overlay status in toolbar."),
    ("UX-029", "medium", "layout", "Setup", "Readiness cards still feel too card-heavy", "Six cards create a dashboard feel where a checklist would be faster.", "Use compact readiness checklist with expandable details."),
    ("UX-030", "medium", "workflow", "Setup", "Method presets do not show what changes", "Preset cards are blank and do not expose settings delta or output implications.", "Make presets selectable cards with expected outputs and setting changes."),
    ("UX-031", "medium", "layout", "Setup", "Advanced methods are visible too early", "MICOM/MGA/CB-SEM/NCA compete with standard PLS setup.", "Place advanced families in Expert accordion groups."),
    ("UX-032", "low", "controls", "Setup", "Bootstrap checkbox feels unfinished", "The control is isolated and not styled as a clear setting row.", "Use a toggle row with sample count and inference consequence."),
    ("UX-033", "medium", "layout", "Setup", "Lower method table is too far down", "Users must scroll to see method status list and settings.", "Use split layout: method selection left, readiness/details right."),
    ("UX-034", "medium", "copy", "Setup", "Validated scope language needs hierarchy", "Validated badges appear, but exact support boundaries are not always near settings.", "Add scope details inline for the selected method only."),
    ("UX-035", "medium", "layout", "Run", "Run page repeats Setup readiness", "The screen repeats readiness cards instead of focusing on execution.", "Compress readiness to a top checklist and expand only failures."),
    ("UX-036", "medium", "workflow", "Run", "Top global Run and page Run compete", "Two run actions are visible for the same outcome.", "One primary action per screen; global action becomes shortcut only."),
    ("UX-037", "medium", "copy", "Run", "Post-run guidance is generic", "After completion, Before publication, Next step cards do not adapt deeply to run state.", "Use result-aware next steps after a completed run."),
    ("UX-038", "low", "layout", "Run", "Run screen is sparse at 1920px", "Cards span large space without enough useful detail.", "Add compact execution summary, output preview, and provenance."),
    ("UX-039", "high", "layout", "Results", "Results tab block wraps awkwardly", "Tabs form a grid and consume too much space before results.", "Use a left section list or compact horizontal tabs with overflow menu."),
    ("UX-040", "high", "tables", "Results", "Tables are hard to scan", "Wide tables, wrapped labels, and nested scrolling hurt readability.", "Build a reusable research table shell with sticky headers, column groups, density, and horizontal affordances."),
    ("UX-041", "high", "interpretation", "Results", "Findings are too many and repetitive", "HTMT and mirrored construct pairs create repeated issue cards.", "Deduplicate symmetrical findings and cap visible cards by priority."),
    ("UX-042", "high", "layout", "Results", "Issue cards dominate the page", "Findings push numerical tables down and overload users visually.", "Summarize findings in Must address / Review / Info lanes with expandable details."),
    ("UX-043", "high", "tables", "Results", "Mediation table is especially difficult", "Important columns overflow and class text wraps awkwardly.", "Split mediation into direct/indirect/total, inference, and classification subtables."),
    ("UX-044", "medium", "workflow", "Results", "Controls are crowded and misaligned", "Search, copy, export, precision, validated-only, interpretation, compact sit in one row.", "Group table tools into toolbar sections and menus."),
    ("UX-045", "medium", "interpretation", "Results", "Interpretation hierarchy needs stronger triage", "Current findings are value-specific but not yet a clear reporting checklist.", "Make Interpretation tab the canonical report-readiness checklist."),
    ("UX-046", "medium", "layout", "Results", "Result summary cards are useful but plain", "Strongest R2, Paths, Warnings cards lack interaction and visual ranking.", "Make summary cards clickable filters into relevant sections."),
    ("UX-047", "medium", "copy", "Results", "Scope warning line is low contrast and repetitive", "The validated-scope sentence appears in many places.", "Use a single run-level scope chip with expandable details."),
    ("UX-048", "high", "diagram", "Report", "Publication diagram still has label overlap", "Customer Satisfaction label/values collide in the preview.", "Apply export label-collision rules and preview fail warnings."),
    ("UX-049", "medium", "layout", "Report", "Publication preview uses nested scrolling", "The preview scrolls inside the page, making export review awkward.", "Use fixed preview frame with fit-to-width/fit-page controls."),
    ("UX-050", "medium", "workflow", "Report", "Export cards look passive", "CSV/HTML/XLSX/PDF cards at the bottom do not clearly behave as actions.", "Convert to explicit export buttons with disabled reasons and output destination feedback."),
    ("UX-051", "medium", "controls", "Report", "Report controls are scattered", "Precision, palette, layout, loadings, path coefficients, R2, interpretation notes are spatially dispersed.", "Group controls into Figure, Statistics, Tables, and Notes sections."),
    ("UX-052", "medium", "workflow", "Report", "Run comparison appears in Report without clear purpose", "Comparison table is useful but seems misplaced below export cards.", "Move comparison to Results Comparison and only link summary from Report."),
    ("UX-053", "medium", "copy", "Report", "PDF wording reads like limitation text", "Browser print-to-PDF is described, but the flow is not guided.", "Provide a visible Print/PDF instruction action or keep it as documentation only."),
    ("UX-054", "medium", "accessibility", "Global", "Contrast of subtle text may be weak", "Light blue/grey backgrounds with small text could fail for some users.", "Run contrast audit and increase secondary text contrast."),
    ("UX-055", "medium", "accessibility", "Global", "Icon-only actions need visible labels or robust tooltips", "Model explorer has several unlabeled icon actions.", "Ensure accessible names, tooltips, and contextual labels for every icon action."),
    ("UX-056", "medium", "accessibility", "Tables", "Keyboard table navigation is not visible from screenshots", "Launch readiness requires proving keyboard traversal and focus states.", "Add keyboard smoke for result/data table search, row focus, and copy/export."),
    ("UX-057", "medium", "performance", "Model", "Large model interaction quality remains unproven visually", "Screens show small sample model only.", "Add large-model visual QA after redesign: 8/32 and 20/80 models."),
    ("UX-058", "medium", "design-system", "Global", "No obvious token audit artifact", "The UI appears to have accumulated local styles over time.", "Create token inventory and forbid one-off card/table/status styling."),
    ("UX-059", "high", "launch", "Global", "No single launch-grade visual acceptance checklist", "Prior gates prove slices, not final visual quality across all screens.", "Create a launch UI QA checklist with required screenshots and pass/fail criteria."),
    ("UX-060", "high", "planning", "Global", "Next milestones need dependency order", "Random fixes could create churn.", "Sequence work as shell/design-system, Results, Report, Model, Setup/Run, Data/Home."),
]


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    SCREEN_DIR.mkdir(parents=True, exist_ok=True)
    copied = []
    missing = []
    for filename, screen, source_text in SCREENSHOTS:
        source = Path(source_text)
        destination = SCREEN_DIR / filename
        if source.exists():
            shutil.copy2(source, destination)
            copied.append({"screen": screen, "path": str(destination.relative_to(ROOT)), "bytes": destination.stat().st_size})
        else:
            missing.append({"screen": screen, "source": source_text})

    issues = [
        {
            "id": issue_id,
            "severity": severity,
            "category": category,
            "screen": screen,
            "title": title,
            "evidence": evidence,
            "recommendation": recommendation,
        }
        for issue_id, severity, category, screen, title, evidence, recommendation in ISSUES
    ]
    severity_counts = Counter(issue["severity"] for issue in issues)
    screen_counts = Counter(issue["screen"] for issue in issues)
    category_counts = Counter(issue["category"] for issue in issues)
    result = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.7 UI/UX launch-quality audit smoke",
        "passed": len(copied) == len(SCREENSHOTS) and len(issues) >= 60,
        "screenshots": copied,
        "missing_screenshots": missing,
        "issue_count": len(issues),
        "severity_counts": dict(severity_counts),
        "screen_counts": dict(screen_counts),
        "category_counts": dict(category_counts),
        "top_priority_issue_ids": [
            "UX-039",
            "UX-040",
            "UX-041",
            "UX-042",
            "UX-048",
            "UX-001",
            "UX-016",
            "UX-022",
            "UX-059",
            "UX-060",
        ],
        "recommended_sequence": [
            "v1_5_8_results_workspace_launch_redesign",
            "v1_5_9_report_publication_workflow_redesign",
            "v1_6_0_model_canvas_shell_and_panel_polish",
            "v1_6_1_setup_run_workflow_consolidation",
            "v1_6_2_data_home_launch_polish",
            "v1_6_3_global_design_system_and_accessibility_pass",
        ],
        "issues": issues,
    }
    OUTPUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
