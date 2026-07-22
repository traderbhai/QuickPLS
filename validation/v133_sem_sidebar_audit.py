import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v133_sem_sidebar_audit.json"
SMOKE = RESULTS / "v133_sem_sidebar_smoke.json"


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
    checklist = {
        "explorer_uses_sem_native_tabs": contains("src/components/Explorer.tsx", ['"constructs"', '"variables"', '"structure"', '"issues"', "explorer-tabs"]),
        "duplicate_data_model_tabs_removed": excludes("src/components/Explorer.tsx", ["pane-tabs", "<button>Data</button><button className=\"active\">Model</button>"]),
        "project_status_and_summary_exist": contains("src/components/Explorer.tsx", ["explorer-status-card", "explorer-summary", "projectName", "dataset.name"]),
        "construct_actions_exist": contains("src/components/Explorer.tsx", ["renameConstruct", "duplicateConstruct", "deleteConstruct", "createPathFrom", "setConstructIndicatorSide", "resetIndicatorLayout", "toggleConstructPinned"]),
        "variable_assignment_workflow_exists": contains("src/components/Explorer.tsx", ["addConstructsFromIndicators", "addConstructsFromIndicatorGroups", "assignIndicators", "assignIndicator", "unassignIndicator", "quickpls:variables-dragging"]),
        "structure_tab_controls_exist": contains("src/components/Explorer.tsx", ["structuralEdges", "covarianceEdges", "reversePath", "setPathRouting", "resetEdgeLabel", "deleteEdge"]),
        "issues_tab_is_actionable": contains("src/components/Explorer.tsx", ["Model issues", "No structural paths", "has no indicators", "Fix"]),
        "sidebar_resize_and_collapse_exist": contains("src/components/Explorer.tsx", ["startResize", "setExplorerWidth", "setExplorerCollapsed", "explorerCollapsed"]),
        "canvas_focus_hooks_exist": contains("src/components/ModelCanvas.tsx", ["quickpls:focus-construct", "quickpls:focus-edge", "setCenter"]),
        "store_ui_state_exists": contains("src/store.ts", ["explorerTab", "explorerCollapsed", "explorerWidth", "setExplorerTab", "setExplorerCollapsed", "setExplorerWidth"]),
        "store_ui_state_tested": contains("src/store.test.ts", ["keeps SEM explorer UI preferences separate from numerical history", "setExplorerTab", "setExplorerWidth", "setExplorerCollapsed"]),
        "css_replaces_dense_tree_with_sem_explorer": contains("src/styles.css", [".sem-explorer", ".explorer-header", ".explorer-status-card", ".explorer-tabs", ".explorer-card", ".structure-row", ".issue-row", ".explorer-resize-handle"]),
        "version_and_release_label_updated": contains("package.json", ['"version": "1.3.3"', "v1_3_3_sem_explorer_sidebar_redesign"]),
        "npm_scripts_registered": contains("package.json", ["qpls:v133:sidebar-smoke", "qpls:v133:sidebar-audit", "qpls:v133:sidebar"]),
        "registry_gate_registered": contains("validation/development_slices.json", ["v1_3_3_sem_explorer_sidebar_redesign", "validation/v133_sem_sidebar_audit.py"]),
        "visual_smoke_passed": json_passed(SMOKE),
        "visual_smoke_covers_key_sidebar_states": all(json_check(SMOKE, key) for key in [
            "tabs_are_model_native",
            "constructs_tab_lists_construct_actions",
            "variables_tab_exposes_assignment_workflow",
            "structure_tab_lists_paths_and_controls",
            "issues_tab_surfaces_actionable_status",
            "collapse_and_expand_workflow",
        ]),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.3.3 SEM explorer sidebar redesign",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "evidence": [
            "src/components/Explorer.tsx",
            "src/components/ModelCanvas.tsx",
            "src/store.ts",
            "src/styles.css",
            "validation/results/v133_sem_sidebar_smoke.json",
            "validation/results/screens/v133/sidebar/",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
