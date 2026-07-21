import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v131_sem_geometry_audit.json"
SMOKE = RESULTS / "v131_sem_geometry_smoke.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def contains(path: str, needles: list[str]) -> bool:
    body = read(path)
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
        "src/domain/semGeometry.ts",
        "src/types.ts",
    ]
    mojibake_hits = [path for path in source_files if "RÃ‚Â²" in read(path) or "RÂ²" in read(path)]
    smoke = json.loads(SMOKE.read_text(encoding="utf-8")) if SMOKE.exists() else {}
    smoke_evidence = smoke.get("evidence", {}).get("fixtures", {})
    checklist = {
        "shared_sem_geometry_module": contains("src/domain/semGeometry.ts", ["boundaryPoint", "routeBetweenBoxes", "measureDiagramQuality", "smartIndicatorPosition"]),
        "canvas_uses_shared_geometry": contains("src/domain/diagramGraph.ts", ["routeBetweenBoxes", "semNodeBox", "smartIndicatorPosition", "measureDiagramQuality"]),
        "svg_uses_shared_geometry": contains("src/domain/publicationDiagram.ts", ["routeBetweenBoxes", "semNodeBox", "SEM_SIZES", 'markerWidth="10"']),
        "default_theme_is_smartpls_like": contains("src/domain/diagramGraph.ts", ['"smartpls_like"']) and contains("src/types.ts", ["smartpls_like", "journal_mono"]),
        "professional_shape_tokens": contains("src/styles.css", ["width: 104px", "height: 68px", "width: 88px", "smartpls-measurement-edge"]),
        "geometry_unit_tests": contains("src/domain/semGeometry.test.ts", ["true ellipse boundary", "nearest rectangle boundary", "measureDiagramQuality"]),
        "label_and_indicator_commands": contains("src/components/ModelCanvas.tsx", ["Tidy labels", "Auto-place indicators", "Tidy selected construct", "resetAllEdgeLabels"]),
        "publication_svg_tests": contains("src/domain/publicationDiagram.test.ts", ['rx="52" ry="34"', "smartpls-edit-handle", "marker-end"]),
        "visual_smoke_passed": json_passed(SMOKE),
        "smoke_has_dense_quality_metrics": all(
            fixture in smoke_evidence and smoke_evidence[fixture].get("after", {}).get("latent_overlap_count") == 0
            for fixture in ["medium", "large", "mediation", "formative"]
        ),
        "no_r2_mojibake": not mojibake_hits,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.3.1 SEM diagram geometry polish",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "mojibake_hits": mojibake_hits,
        "evidence": [
            "src/domain/semGeometry.ts",
            "src/domain/semGeometry.test.ts",
            "validation/results/v131_sem_geometry_smoke.json",
            "validation/results/screens/v131/sem-geometry/",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
