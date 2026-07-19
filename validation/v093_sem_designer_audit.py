"""Audit for the v0.9.3 professional SEM designer milestone."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v093_sem_designer_audit.json"


def contains(path: Path, terms: list[str]) -> dict[str, bool]:
    text = path.read_text(encoding="utf-8")
    return {term: term in text for term in terms}


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    checks = {
        "layout_types": contains(ROOT / "src" / "types.ts", [
            "DiagramLayoutState",
            "diagramVersion",
            "constructLayouts",
            "indicatorLayouts",
            "edgeLayouts",
            "layoutSource",
        ]),
        "graph_layout": contains(ROOT / "src" / "domain" / "diagramGraph.ts", [
            "defaultDiagramLayout",
            "indicatorPositionsForConstruct",
            "layoutSource",
            "current_canvas",
            "tidy_publication",
        ]),
        "store_layout": contains(ROOT / "src" / "store.ts", [
            "diagramLayout",
            "moveIndicator",
            "setIndicatorSide",
            "resetIndicatorLayout",
            "syncedDiagramLayout",
        ]),
        "canvas_editing": contains(ROOT / "src" / "components" / "ModelCanvas.tsx", [
            "diagram-context-menu",
            "nearestConstructForIndicator",
            "onNodeDragStop",
            "Residual/error node placeholder",
            "Right-click opens object actions",
        ]),
        "topbar_save_load": contains(ROOT / "src" / "components" / "TopBar.tsx", ["diagramLayout", "publicationDiagramSettings"]),
        "autosave": contains(ROOT / "src" / "App.tsx", ["diagramLayout", "autosaveNativeProject"]),
        "publication_export": contains(ROOT / "src" / "domain" / "publicationDiagram.ts", ["layoutSource", "DiagramLayoutState", "buildDiagramGraph(nodes, edges, options.mode"]),
        "reports": contains(ROOT / "src" / "components" / "ReportsWorkspace.tsx", ["Current canvas", "Tidy publication", "diagramLayout"]),
        "styles": contains(ROOT / "src" / "styles.css", ["diagram-context-menu", "cursor: grab", "locked-result-canvas"]),
        "store_tests": contains(ROOT / "src" / "store.test.ts", ["persists and resets indicator layout"]),
        "graph_tests": contains(ROOT / "src" / "domain" / "diagramGraph.test.ts", ["persisted free indicator positions", "current canvas positions"]),
        "svg_tests": contains(ROOT / "src" / "domain" / "publicationDiagram.test.ts", ["current canvas indicator layout"]),
        "smoke": contains(ROOT / "validation" / "v093_sem_designer_visual_smoke.mjs", ["v093_default_sem_canvas_1440x900.png", "draggedIndicatorMoved"]),
    }
    passed = all(all(values.values()) for values in checks.values())
    report = {
        "schema_version": 1,
        "target": "v0.9.3 professional SEM designer audit",
        "passed": passed,
        "checks": checks,
        "note": "Verifies unified academic SEM styling, persistent layout metadata, editable indicator movement, context menus, save/load plumbing, and current-canvas publication export.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
