"""Final v1.0 product scope enforcement audit."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v10_product_scope_audit.json"
VERSION = "1.0.0"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def no_forbidden_claims(text: str) -> bool:
    lowered = text.lower()
    forbidden = [
        "identical to smartpls",
        "guaranteed accurate",
        "reverse engineered",
        "decompiled",
    ]
    return not any(term in lowered for term in forbidden)


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    topbar = read(ROOT / "src" / "components" / "TopBar.tsx")
    publication = read(ROOT / "src" / "domain" / "publicationDiagram.ts")
    result_tables = read(ROOT / "src" / "domain" / "resultTables.ts")
    reports = read(ROOT / "src" / "components" / "ReportsWorkspace.tsx")
    stable_export = read_json(RESULTS / "stable_export_publication_audit.json")
    designer = read_json(RESULTS / "v093_sem_designer_audit.json")
    designer_smoke = read_json(RESULTS / "v093_sem_designer_visual_smoke.json")
    docs = "\n".join(
        read(path)
        for path in [
            ROOT / "docs" / "V1_SUPPORTED_SCOPE.md",
            ROOT / "docs" / "V1_COMPATIBILITY_MATRIX.md",
            ROOT / "docs" / "V1_KNOWN_DIFFERENCES.md",
            ROOT / "docs" / "RELEASE_NOTES_V1_0.md",
        ]
    )
    checks = {
        "ui_version_label_v100": 'className="alpha-mark">v1.0.0' in topbar,
        "run_warning_v100_scope": "QuickPLS v1.0.0 supported scope" in topbar,
        "diagram_watermark_v100_scope": "QuickPLS v1.0.0 supported scope" in publication,
        "report_tables_v100_scope": "QuickPLS v1.0.0 supported scope" in result_tables,
        "stable_exports_audited": stable_export.get("passed") is True,
        "experimental_exports_opt_in": "include_experimental" in read(ROOT / "crates" / "qpls-cli" / "src" / "main.rs") and "watermarked experimental" in read(ROOT / "crates" / "qpls-cli" / "src" / "main.rs"),
        "report_surface_has_provenance_and_warning": "Run provenance" in result_tables and "Warning" in result_tables,
        "diagram_estimates_selected_run_guard": "resultForOverlay" in read(ROOT / "src" / "domain" / "diagramGraph.ts") and "resultMatchesModel" in read(ROOT / "src" / "domain" / "diagramGraph.ts"),
        "v093_designer_audit_passed": designer.get("passed") is True,
        "v093_visual_smoke_passed": designer_smoke.get("passed") is True,
        "no_forbidden_source_or_doc_claims": no_forbidden_claims(topbar + publication + result_tables + reports + docs),
        "docs_state_no_smartpls_import": "SmartPLS project files are not imported" in docs,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.0.0 product scope audit",
        "passed": all(checks.values()),
        "checks": checks,
        "note": "Default product surfaces are constrained to the v1.0 supported scope; experimental or unsupported outputs remain opt-in, watermarked, or blocked.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
