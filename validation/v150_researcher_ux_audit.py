import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v150_researcher_ux_audit.json"
SMOKE = RESULTS / "v150_researcher_ux_smoke.json"


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def contains(path: str, needles: list[str]) -> bool:
    body = text(path)
    return all(needle in body for needle in needles)


def excludes(path: str, needles: list[str]) -> bool:
    body = text(path)
    return all(needle not in body for needle in needles)


def json_passed(path: Path) -> bool:
    return path.exists() and bool(json.loads(path.read_text(encoding="utf-8")).get("passed"))


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    checklist = {
        "command_palette_and_shortcuts": contains("src/components/ProductivityOverlays.tsx", ["Ctrl+K", "Quick actions", "Keyboard shortcuts", "Show keyboard shortcuts"]),
        "toast_feedback_layer": contains("src/components/ProductivityOverlays.tsx", ["toast-stack", "Application notifications"]) and contains("src/components/TopBar.tsx", ["pushToast", "Project saved", "Dataset imported", "Run completed"]),
        "status_bar_desktop_feedback": contains("src/components/StatusBar.tsx", ["Autosave active", "Save project to enable autosave", "Shortcuts ?"]),
        "method_what_will_run_summary": contains("src/components/AnalysisCatalog.tsx", ["What will run", "Bootstrap", "Permutation", "Validated documented scope"]),
        "results_headline_and_export_tools": contains("src/components/RunHistory.tsx", ["result-headline-grid", "Strongest R²", "Export current table", "Click a row to focus the diagram"]),
        "publication_export_stepper": contains("src/components/ReportsWorkspace.tsx", ["export-stepper", "Select run", "Choose diagram style", "Preview figure", "Export tables and SVG"]),
        "explorer_prefix_grouping": contains("src/components/Explorer.tsx", ["prefixSummary", "Prefix groups", "Detected variable prefix groups"]),
        "styles_cover_new_surfaces": contains("src/styles.css", [".command-palette", ".shortcut-panel", ".toast-stack", ".what-will-run-card", ".result-headline-grid", ".export-stepper", ".prefix-chip-row"]),
        "version_and_artifact_label": contains("package.json", ['"version": "1.5.0"', "v1_5_0_researcher_ux_refinement"]) and contains("src-tauri/tauri.conf.json", ['"version": "1.5.0"']),
        "no_ui_mojibake": all(excludes(path, ["RÂ²"]) for path in ["src/components/ModelCanvas.tsx", "src/components/RunHistory.tsx", "src/components/ReportsWorkspace.tsx", "src/components/ProductivityOverlays.tsx", "src/styles.css"]),
        "visual_smoke_passed": json_passed(SMOKE),
        "registry_gate_registered": contains("validation/development_slices.json", ["v1_5_0_researcher_ux_refinement", "validation/v150_researcher_ux_audit.py"]),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.0 researcher UX refinement",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "evidence": [
            "src/components/ProductivityOverlays.tsx",
            "src/components/AnalysisCatalog.tsx",
            "src/components/RunHistory.tsx",
            "src/components/ReportsWorkspace.tsx",
            "src/components/Explorer.tsx",
            "src/styles.css",
            "validation/results/v150_researcher_ux_smoke.json",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
