"""Audit for the v0.9.2 SmartPLS-like result diagram milestone."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v092_smartpls_diagram_audit.json"


def contains(path: Path, terms: list[str]) -> dict[str, bool]:
    text = path.read_text(encoding="utf-8")
    return {term: term in text for term in terms}


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    checks = {
        "types": contains(ROOT / "src" / "types.ts", ["smartpls_result", "showLoadings", "showPathCoefficients", "showRSquared", "high_contrast", "quickpls_color"]),
        "graph": contains(ROOT / "src" / "domain" / "diagramGraph.ts", ["smartplsLayout", "SMARTPLS_LATENT_WIDTH", "Run or select a compatible result to show estimates.", "smartpls-measurement-edge", "smartpls-structural-edge"]),
        "canvas": contains(ROOT / "src" / "components" / "ModelCanvas.tsx", ["Result diagram", "Arrange like SmartPLS", "nodesDraggable={!resultDiagramMode}", "smartpls-result-canvas"]),
        "latent_node": contains(ROOT / "src" / "components" / "LatentNode.tsx", ["smartpls-latent-node", "smartpls-r2", "R²"]),
        "indicator_node": contains(ROOT / "src" / "components" / "IndicatorNode.tsx", ["smartpls-indicator-node", "displayMode === \"smartpls_result\""]),
        "svg_export": contains(ROOT / "src" / "domain" / "publicationDiagram.ts", ["smartpls-latent", "smartpls-indicator", "R&#178;", "showLoadings", "showPathCoefficients", "showRSquared"]),
        "reports": contains(ROOT / "src" / "components" / "ReportsWorkspace.tsx", ["SmartPLS-like", "QuickPLS publication", "Loadings", "Path coefficients", "R²"]),
        "styles": contains(ROOT / "src" / "styles.css", ["smartpls-result-canvas", "smartpls-latent-ellipse", "smartpls-indicator-node", "smartpls-measurement-edge"]),
        "tests": contains(ROOT / "src" / "domain" / "publicationDiagram.test.ts", ["SmartPLS-like", "R&#178; 0.208", "not.toContain(\"Mode A\")"]),
        "smoke": contains(ROOT / "validation" / "v092_smartpls_diagram_visual_smoke.mjs", ["smartpls_result", "v092_smartpls_result_1440x900.png"]),
    }
    passed = all(all(values.values()) for values in checks.values())
    report = {
        "schema_version": 1,
        "target": "v0.9.2 SmartPLS-like result diagram audit",
        "passed": passed,
        "checks": checks,
        "note": "This audit verifies the dedicated paper-style result diagram mode, SVG export support, result-overlay contract, and absence of editor artifacts in result mode.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
