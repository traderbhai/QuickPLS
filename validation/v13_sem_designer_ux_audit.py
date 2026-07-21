import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v13_sem_designer_ux_audit.json"
SMOKE = RESULTS / "v13_sem_designer_ux_smoke.json"


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def contains(path: str, needles: list[str]) -> bool:
    body = text(path)
    return all(needle in body for needle in needles)


def json_passed(path: Path) -> bool:
    if not path.exists():
        return False
    return bool(json.loads(path.read_text(encoding="utf-8")).get("passed"))


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    source_files = [
        "src/components/ModelCanvas.tsx",
        "src/components/LatentNode.tsx",
        "src/components/IndicatorNode.tsx",
        "src/components/SemEdge.tsx",
        "src/domain/diagramGraph.ts",
        "src/domain/publicationDiagram.ts",
    ]
    mojibake_hits = [path for path in source_files if "RÂ²" in text(path)]
    checklist = {
        "editable_sem_handles": contains("src/components/LatentNode.tsx", ["editablePaperMode", "smartpls-edit-handle", "source-right", "target-left"]),
        "result_and_publication_locked": contains("src/domain/diagramGraph.ts", ['mode === "smartpls_result" || mode === "publication"', "draggable: !lockedResultMode"]),
        "context_menus_cover_common_actions": contains("src/components/ModelCanvas.tsx", ["Rename construct", "Reset indicator layout", "Reverse path", "Reset label", "Convert to covariance display"]),
        "invalid_path_feedback": contains("src/components/ModelCanvas.tsx", ["canvas-action-feedback", "Self-paths", "already exists", "createPathOrCovariance"]),
        "edge_labels_drag_and_keyboard": contains("src/components/SemEdge.tsx", ["onPointerDown={startDrag}", "onKeyDown={handleKeyDown}", "ArrowUp", "Home resets"]),
        "publication_svg_uses_shared_grammar": contains("src/domain/publicationDiagram.ts", ["smartpls-latent", "smartpls-indicator", "smartpls-r2", "layoutSource"]),
        "visual_smoke_passed": json_passed(SMOKE),
        "no_source_r2_mojibake": not mojibake_hits,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.3 SEM designer UX overhaul",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "mojibake_hits": mojibake_hits,
        "evidence": [
            "validation/results/v13_sem_designer_ux_smoke.json",
            "validation/results/screens/v13/sem-designer/01_editable_academic_canvas.png",
            "validation/results/screens/v13/sem-designer/02_arranged_sem_canvas.png",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
