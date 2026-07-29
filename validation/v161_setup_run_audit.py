import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


checks = []


def check(name: str, passed: bool, evidence: str) -> None:
    checks.append({"name": name, "passed": bool(passed), "evidence": evidence})


setup = read("src/components/AnalysisCatalog.tsx")
run = read("src/components/RunWorkspace.tsx")
styles = read("src/styles.css")
registry = read("validation/development_slices.json")
docs = read("docs/V1_6_1_SETUP_RUN_WORKFLOW_CONSOLIDATION.md")

check(
    "Setup launches the configured run directly",
    "quickpls:run-analysis" in setup and "Run selected method" in setup and "Open run monitor" in setup,
    "AnalysisCatalog dispatches the production run event and keeps the run monitor as a secondary destination.",
)
check(
    "Setup has a consolidated launch panel",
    "setup-launch-panel" in setup and "Ready-to-run summary" in setup and "setup-launch-panel" in styles,
    "Setup combines run summary, scope, resampling state, and launch action in one panel.",
)
check(
    "Setup removes duplicate readiness/run-state cards",
    "title: \"Readiness\"" not in setup and "title: \"Run state\"" not in setup,
    "Setup keeps readiness in the readiness panel and does not repeat the same summary as extra cards.",
)
check(
    "Run workspace no longer repeats the full readiness panel",
    "ReadinessPanel" not in run and "run-monitor-summary" in run,
    "RunWorkspace uses a compact monitor summary instead of rendering the full readiness grid again.",
)
check(
    "Run routes settings changes back to Setup",
    "Need to change settings?" in run and "Open setup" in run and 'setView("analyses")' in run,
    "Run remains focused on execution and links configuration edits back to Setup.",
)
check(
    "Registry and docs are wired",
    "v1_6_1_setup_run_workflow_consolidation" in registry and "v1.6.1 Setup/Run workflow consolidation" in docs,
    "Development slice registry and milestone documentation reference v1.6.1.",
)
check(
    "No mojibake introduced",
    "RÃ‚" not in setup + run and "Ã‚" not in setup + run and "Ã¢" not in setup + run,
    "Setup/Run sources contain no visible mojibake sequences.",
)

passed = all(item["passed"] for item in checks)
result = {
    "passed": passed,
    "milestone": "v1_6_1_setup_run_workflow_consolidation",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "checks": checks,
}

out = ROOT / "validation" / "results" / "v161_setup_run_audit.json"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(result, indent=2), encoding="utf-8")

if not passed:
    for item in checks:
        if not item["passed"]:
            print(f"FAILED: {item['name']} - {item['evidence']}")
    raise SystemExit(1)

print("v1.6.1 setup/run audit passed")
