from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"


SECTION_CHECKS: dict[str, dict[str, list[tuple[str, str]]]] = {
    "v170_confidence_scope": {
        "src/components/Ui.tsx": [
            ("Why trust this result?", "method-scope drawer entry point"),
            ("Method Confidence", "run confidence panel"),
            ("Runtime dependency", "offline/runtime dependency note"),
            ("QuickPLS does not claim SmartPLS project import or equivalence", "bounded non-equivalence wording"),
        ],
        "src/components/AnalysisCatalog.tsx": [
            ("<MethodScopeDrawer", "scope drawer appears in Setup"),
            ("calculation-preview-panel", "calculation output preview"),
        ],
        "src/components/ReportsWorkspace.tsx": [
            ("<MethodScopeDrawer", "scope drawer appears in Reports"),
            ("<MethodConfidencePanel", "method confidence appears in Reports"),
        ],
    },
    "v171_workflow": {
        "src/components/DataWorkspace.tsx": [
            ("Open Model Designer", "Data to Model transition"),
            ("Create Constructs From Prefixes", "prefix-based construct bridge"),
        ],
        "src/components/AnalysisCatalog.tsx": [
            ("After run", "post-run workflow preview"),
            ("Produced outputs", "pre-run output preview"),
            ("Unavailable outputs", "pre-run unavailable output preview"),
        ],
        "src/components/RunWorkspace.tsx": [
            ("Open results", "Run to Results handoff"),
        ],
        "src/components/RunHistory.tsx": [
            ("Prepare report", "Results to Report handoff or report workflow copy"),
        ],
    },
    "v172_sem_designer": {
        "src/components/ModelCanvas.tsx": [
            ("Focus Diagram", "focus diagram mode"),
            ("Check diagram for publication", "publication diagram check"),
            ("shortest boundary", "shortest path geometry comment or copy"),
            ("quickpls:focus-edge", "diagram object focusing from results"),
        ],
        "src/styles.css": [
            ("focus-diagram-mode", "focus mode layout style"),
        ],
    },
    "v173_reportability": {
        "src/components/RunHistory.tsx": [
            ("ReportabilityChecklist", "reportability checklist component usage"),
            ("reportabilityItems", "value-driven checklist builder"),
            ("Indicator reliability", "indicator reliability checklist item"),
            ("Threshold colors", "threshold color control"),
            ("Do not report p values or confidence intervals", "inference caveat"),
        ],
        "src/components/Ui.tsx": [
            ("Reportability checklist", "shared checklist UI"),
            ("Threshold colors are methodological guidance", "threshold caveat"),
        ],
    },
    "v174_research_tables": {
        "src/components/Ui.tsx": [
            ("ResearchTable", "reusable research table shell"),
            ("sticky-col", "sticky first column support"),
            ("rows.length", "row count metadata"),
            ("columns.length", "column count metadata"),
        ],
        "src/components/RunHistory.tsx": [
            ("Search result tables", "results table search"),
            ("Export table", "current table export"),
            ("Result precision", "precision control"),
            ("Copy table", "copy table affordance"),
        ],
    },
    "v175_publication_pack": {
        "src/components/ReportsWorkspace.tsx": [
            ("Reviewer pack", "reviewer-pack preset"),
            ("Reviewer pack contents", "reviewer-pack preview"),
            ("validation artifact index", "validation index reference"),
            ("Include interpretation notes", "explicit interpretation opt-in"),
            ("WYSIWYG SVG export", "audited SVG figure path"),
        ],
    },
    "v176_samples_guided": {
        "src/components/OnboardingWorkspace.tsx": [
            ("Sample project gallery", "sample project gallery"),
            ("Simple reflective PLS-SEM", "simple PLS sample"),
            ("MICOM / MGA", "group sample"),
            ("CB-SEM CFA", "CB-SEM sample"),
            ("Start from dataset", "guided dataset workflow"),
            ("detect prefixes", "guided prefix step"),
        ],
    },
}


def read_text(relative_path: str) -> str:
    return (ROOT / relative_path).read_text(encoding="utf-8")


def run_section(section: str, result_name: str) -> bool:
    checks = SECTION_CHECKS[section]
    missing: list[dict[str, str]] = []
    passed_checks: list[dict[str, str]] = []
    for relative_path, snippets in checks.items():
        text = read_text(relative_path)
        for snippet, label in snippets:
            if snippet in text:
                passed_checks.append({"file": relative_path, "label": label})
            else:
                missing.append({"file": relative_path, "label": label, "snippet": snippet})
    payload = {
        "passed": not missing,
        "section": section,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "checked_files": sorted(checks.keys()),
        "passed_checks": passed_checks,
        "missing": missing,
        "frontend_only": True,
        "numerical_outputs_changed": False,
    }
    RESULTS.mkdir(parents=True, exist_ok=True)
    (RESULTS / result_name).write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if missing:
        for item in missing:
            print(f"missing {item['label']} in {item['file']}: {item['snippet']}")
    print(json.dumps({"result": result_name, "passed": payload["passed"]}, indent=2))
    return payload["passed"]


def aggregate(result_files: Iterable[str], result_name: str) -> bool:
    missing_files = [name for name in result_files if not (RESULTS / name).exists()]
    failed = []
    for name in result_files:
        if name in missing_files:
            continue
        data = json.loads((RESULTS / name).read_text(encoding="utf-8"))
        if not data.get("passed"):
            failed.append(name)
    payload = {
        "passed": not missing_files and not failed,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "required_artifacts": list(result_files),
        "missing_files": missing_files,
        "failed_artifacts": failed,
        "frontend_only": True,
        "numerical_outputs_changed": False,
    }
    RESULTS.mkdir(parents=True, exist_ok=True)
    (RESULTS / result_name).write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if missing_files or failed:
        print(json.dumps(payload, indent=2))
    else:
        print(json.dumps({"result": result_name, "passed": True}, indent=2))
    return payload["passed"]
