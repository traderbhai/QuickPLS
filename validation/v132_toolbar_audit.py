import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v132_toolbar_audit.json"
SMOKE = RESULTS / "v132_toolbar_smoke.json"


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def contains(path: str, needles: list[str]) -> bool:
    body = text(path)
    return all(needle in body for needle in needles)


def excludes(path: str, needles: list[str]) -> bool:
    body = text(path)
    return all(needle not in body for needle in needles)


def json_passed(path: Path) -> bool:
    if not path.exists():
        return False
    return bool(json.loads(path.read_text(encoding="utf-8")).get("passed"))


def json_check(path: Path, key: str) -> bool:
    if not path.exists():
        return False
    return bool(json.loads(path.read_text(encoding="utf-8")).get("checklist", {}).get(key))


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    source_files = [
        "src/components/ModelCanvas.tsx",
        "src/components/LatentNode.tsx",
        "src/components/ReportsWorkspace.tsx",
        "src/domain/publicationDiagram.ts",
    ]
    mojibake_hits = [path for path in source_files if "RÂ²" in text(path) or "RÃ‚Â²" in text(path)]
    checklist = {
        "toolbar_three_zone_structure": contains("src/components/ModelCanvas.tsx", ["canvas-toolbar-primary", "canvas-context-toolbar", "canvas-dropdown-menu", "result-tools menu-group"]),
        "main_toolbar_core_actions": contains("src/components/ModelCanvas.tsx", ["Undo", "Redo", "Select", "Pan", "Construct", "Path", "Cov", "Arrange", "Fit", "Validate", "View", "Results"]),
        "placeholder_tools_hidden_from_main_toolbar": excludes("src/components/ModelCanvas.tsx", ["Residual or error node tool", "Caption tool", "Observed indicator tool", "Latent construct tool"]),
        "arrange_view_results_dropdowns": contains("src/components/ModelCanvas.tsx", ["Arrange like SmartPLS", "Left to right", "Top to bottom", "Diagram mode", "Diagram result run", "Diagram result overlay", "Diagram overlay precision"]),
        "view_controls_are_functional": contains("src/components/ModelCanvas.tsx", ["setDiagramTheme", "setDiagramGridVisible", "setDiagramLayoutLocked", "Journal mono", "High contrast"]) and contains("src/store.ts", ["setDiagramTheme", "setDiagramGridVisible", "setDiagramLayoutLocked"]),
        "construct_context_actions": contains("src/components/ModelCanvas.tsx", ["Selected construct actions", "Auto indicators", "Reset indicator layout", "Indicators left", "Tidy selected"]),
        "indicator_context_actions": contains("src/components/ModelCanvas.tsx", ["Selected indicator actions", "Reset position", "Unassign", "selectIndicatorForToolbar", "reassignSelectedIndicator"]),
        "path_context_actions": contains("src/components/ModelCanvas.tsx", ["Selected path actions", "Reverse", "Straight", "Curved", "Orthogonal", "Reset label", "Mark control"]),
        "multi_selection_context_actions": contains("src/components/ModelCanvas.tsx", ["Selected constructs alignment actions", "Distribute H", "Distribute V", "Tidy selection"]),
        "disabled_reasons_visible": contains("src/components/ModelCanvas.tsx", ["Result view is locked", "Covariance arcs have no structural direction", "no compatible result", "Layout lock is on", "disabledActionReason"]),
        "toolbar_css_no_horizontal_scroll": contains("src/styles.css", [".canvas-toolbar-primary", "overflow: visible", ".canvas-dropdown-menu", ".canvas-context-toolbar"]),
        "theme_css_exists": contains("src/styles.css", ["theme-academic_grayscale", "theme-quickpls_color", "theme-journal_mono", "theme-high_contrast"]),
        "visual_smoke_passed": json_passed(SMOKE),
        "visual_smoke_exercises_functional_view_controls": all(json_check(SMOKE, key) for key in ["view_theme_button_changes_canvas_class", "view_grid_button_hides_grid_and_minimap", "view_lock_button_disables_layout_actions", "multi_selection_toolbar_visible"]),
        "no_r2_mojibake": not mojibake_hits,
        "npm_scripts_registered": contains("package.json", ["qpls:v132:toolbar-smoke", "qpls:v132:toolbar-audit", "qpls:v132:toolbar"]),
        "registry_gate_registered": contains("validation/development_slices.json", ["v1_3_2_sem_canvas_toolbar_redesign", "validation/v132_toolbar_audit.py"]),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.3.2 SEM canvas toolbar redesign",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "mojibake_hits": mojibake_hits,
        "evidence": [
            "validation/results/v132_toolbar_smoke.json",
            "validation/results/screens/v132/toolbar/",
            "src/components/ModelCanvas.tsx",
            "src/styles.css",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
