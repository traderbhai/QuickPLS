import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"


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


SECTIONS = {
    "design-system": {
        "target": "v1_4_0_frontend_design_system_foundation",
        "checks": {
            "shared_ui_components": lambda: contains("src/components/Ui.tsx", ["PageHeader", "StatusBadge", "ActionStrip", "Card", "TabStrip", "EmptyState"]),
            "desktop_density_tokens": lambda: contains("src/styles.css", [".page-heading-pro", ".ui-action-strip", ".ui-card", ".ui-tab-strip", ".density-compact"]),
            "status_badges_standardized": lambda: contains("src/styles.css", [".status-badge.validated", ".status-badge.experimental", ".status-badge.unsupported", ".status-badge.warning"]),
            "mojibake_blocked": lambda: all(excludes(path, ["RÂ²"]) for path in ["src/components/ModelCanvas.tsx", "src/components/ReportsWorkspace.tsx", "src/components/RunHistory.tsx", "src/styles.css"]),
            "release_metadata_updated": lambda: contains("package.json", ['"version": "1.4.7"', "v1_4_frontend_success_program"]) and contains("src-tauri/tauri.conf.json", ['"version": "1.4.7"']),
        },
    },
    "sem-designer": {
        "target": "v1_4_1_sem_designer_completion",
        "checks": {
            "professional_toolbar_remains": lambda: contains("src/components/ModelCanvas.tsx", ["canvas-toolbar-primary", "Path", "Cov", "Arrange", "Fit", "Validate"]),
            "large_model_view_controls": lambda: contains("src/components/ModelCanvas.tsx", ["Collapse measurement indicators", "Isolate selected neighborhood", "Clear isolation", "Fit selected"]),
            "result_modes_locked": lambda: contains("src/components/ModelCanvas.tsx", ["Result view is locked", "resultDiagramMode", "canEditLayout"]),
            "straight_route_default": lambda: contains("src/components/ModelCanvas.tsx", ['return "straight"', "setSelectedPathRouting(\"straight\")"]),
        },
    },
    "explorer-inspector": {
        "target": "v1_4_2_explorer_inspector_simplification",
        "checks": {
            "global_search_exists": lambda: contains("src/components/Explorer.tsx", ["globalMatches", "explorer-global-results", "Search all"]),
            "issue_filters_exist": lambda: contains("src/components/Explorer.tsx", ["issueFilter", "blocking", "warning", "info"]),
            "inspector_progressive_sections": lambda: contains("src/components/Inspector.tsx", ["Essentials", "Layout", "Advanced", "Results"]),
            "common_actions_outside_advanced": lambda: contains("src/components/ModelCanvas.tsx", ["Rename", "Delete", "Reverse", "Reset indicator layout"]),
        },
    },
    "method-setup": {
        "target": "v1_4_3_method_setup_experience",
        "checks": {
            "basic_expert_modes": lambda: contains("src/components/AnalysisCatalog.tsx", ["Basic", "Expert", "methodSetupState", "setMethodSetupState"]),
            "method_presets": lambda: contains("src/components/AnalysisCatalog.tsx", ["Standard PLS-SEM", "PLS + Bootstrap", "PLSpredict", "MICOM + MGA", "CB-SEM CFA", "OLS Regression", "NCA"]),
            "readiness_cards": lambda: contains("src/components/AnalysisCatalog.tsx", ["Missing dataset", "Unsupported shape", "Experimental scope", "Readiness"]),
            "store_preset_actions": lambda: contains("src/store.ts", ["applyMethodPreset", "pls_bootstrap", "micom_mga", "cbsem_cfa"]),
        },
    },
    "results-workspace": {
        "target": "v1_4_4_results_workspace_redesign",
        "checks": {
            "researcher_tabs": lambda: contains("src/components/RunHistory.tsx", ["Summary", "Measurement Model", "Structural Model", "Reliability and Validity", "Inference", "Prediction", "Groups", "Diagnostics", "Comparison"]),
            "table_tools": lambda: contains("src/components/RunHistory.tsx", ["result-search", "copyVisibleSummary", "tableDensity", "includeExperimental"]),
            "diagram_cross_focus": lambda: contains("src/components/RunHistory.tsx", ["focusPath", "quickpls:focus-edge", "quickpls:focus-construct"]),
            "persistent_state": lambda: contains("src/store.ts", ["resultWorkspaceState", "setResultWorkspaceState"]),
        },
    },
    "publication-export": {
        "target": "v1_4_5_publication_export_workflow",
        "checks": {
            "export_presets": lambda: contains("src/components/ReportsWorkspace.tsx", ["Thesis appendix", "Journal figure", "Journal tables", "Presentation", "Full reproducibility report"]),
            "wysiwyg_controls": lambda: contains("src/components/ReportsWorkspace.tsx", ["Current canvas", "Tidy publication", "Diagram style", "Precision"]),
            "audited_svg_scope": lambda: contains("src/components/ReportsWorkspace.tsx", ["SVG", "browser print", "PDF"]),
        },
    },
    "onboarding-demo": {
        "target": "v1_4_6_onboarding_demo_workflow",
        "checks": {
            "welcome_view": lambda: contains("src/types.ts", ['"welcome"']) and contains("src/App.tsx", ["OnboardingWorkspace", 'view === "welcome"']),
            "start_actions": lambda: contains("src/components/OnboardingWorkspace.tsx", ["Start new project", "Open existing project", "Open demo project", "Import dataset", "Continue recent project"]),
            "native_events": lambda: contains("src/components/TopBar.tsx", ["quickpls:open-project", "quickpls:open-demo-project"]),
            "nav_entry": lambda: contains("src/components/NavRail.tsx", ["Start", "Home"]),
        },
    },
    "large-model-desktop": {
        "target": "v1_4_7_large_model_desktop_polish",
        "checks": {
            "large_model_state": lambda: contains("src/types.ts", ["LargeModelViewState", "indicatorsCollapsed", "isolatedConstructId", "neighborhoodMode"]),
            "view_menu_controls": lambda: contains("src/components/ModelCanvas.tsx", ["Collapse measurement indicators", "Isolate selected neighborhood", "Fit selected", "Lock layout"]),
            "smoke_api_supports_welcome": lambda: contains("src/App.tsx", ['"welcome"', "__QUICKPLS_SMOKE__"]),
            "versioned_desktop_build_script": lambda: contains("package.json", ["qpls:desktop:build-versioned", "v1_4_frontend_success_program"]),
        },
    },
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--section", choices=SECTIONS.keys(), required=True)
    args = parser.parse_args()
    RESULTS.mkdir(parents=True, exist_ok=True)
    spec = SECTIONS[args.section]
    checklist = {name: check() for name, check in spec["checks"].items()}
    report = {
        "schema_version": 1,
        "target": spec["target"],
        "section": args.section,
        "passed": all(checklist.values()),
        "checklist": checklist,
        "evidence": ["src/components", "src/store.ts", "src/types.ts", "src/styles.css", "package.json"],
    }
    output = RESULTS / f"{spec['target']}_audit.json"
    output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
