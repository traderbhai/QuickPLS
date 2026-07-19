"""Publication audit for GUI and diagram workflow."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "gui_diagram_publication_audit.json"


def run(command, timeout=300):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {"command": command, "returncode": proc.returncode, "passed": proc.returncode == 0, "stdout_tail": proc.stdout[-3000:], "stderr_tail": proc.stderr[-3000:]}


def contains(path, terms):
    text = path.read_text(encoding="utf-8")
    return {term: term in text for term in terms}


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    npm_test = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "test", "--", "--run"])
    build = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "run", "build"])
    desktop = run(["cargo", "test", "-p", "quickpls-desktop"])
    canvas_source = ROOT / "src" / "components" / "ModelCanvas.tsx"
    diagram_graph = ROOT / "src" / "domain" / "diagramGraph.ts"
    reports = ROOT / "src" / "components" / "ReportsWorkspace.tsx"
    diagram = ROOT / "src" / "domain" / "publicationDiagram.ts"
    topbar = ROOT / "src" / "components" / "TopBar.tsx"
    source_checks = {
        "react_flow_canvas": contains(canvas_source, ["ReactFlow", "buildDiagramGraph", "Diagram result overlay", "Result diagram", "Draw covariance display arc"]),
        "derived_sem_graph": contains(diagram_graph, ["indicatorNodeId", "measurement-edge reflective", "measurement-edge formative", "Selected run does not match the current model"]),
        "diagram_exports": contains(reports, ["publicationDiagramSvg", "quickpls-publication-diagram.svg", "Print / PDF", "Diagram precision", "Diagram palette"]),
        "diagram_estimate_visibility": contains(diagram, ["buildDiagramGraph", "Validated for documented QuickPLS v1.0.0 supported scope", "class=\"latent", "class=\"indicator"]),
        "run_estimate_binding": contains(topbar, ["addRun", "selectedMethod.name", "unsupported shapes remain blocked", "structuralEdges"]),
    }
    test_files = {
        "publication_diagram_test": (ROOT / "src" / "domain" / "publicationDiagram.test.ts").exists(),
        "diagram_graph_test": (ROOT / "src" / "domain" / "diagramGraph.test.ts").exists(),
        "model_layout_test": (ROOT / "src" / "domain" / "modelLayout.test.ts").exists(),
        "result_tables_test": (ROOT / "src" / "domain" / "resultTables.test.ts").exists(),
        "store_test": (ROOT / "src" / "store.test.ts").exists(),
    }
    source_passed = all(all(values.values()) for values in source_checks.values())
    passed = npm_test["passed"] and build["passed"] and desktop["passed"] and source_passed and all(test_files.values())
    report = {
        "schema_version": 1,
        "target": "GUI and diagram publication audit",
        "passed": passed,
        "source_checks": source_checks,
        "test_files": test_files,
        "commands": [npm_test, build, desktop],
        "note": "This audit verifies the implemented desktop/editor/report diagram workflow and result-estimate visibility contract.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
