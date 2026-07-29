import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


checks = []


def check(name: str, passed: bool, evidence: str) -> None:
    checks.append({"name": name, "passed": bool(passed), "evidence": evidence})


home = read("src/components/OnboardingWorkspace.tsx")
data = read("src/components/DataWorkspace.tsx")
styles = read("src/styles.css")
registry = read("validation/development_slices.json")
docs = read("docs/V1_6_2_DATA_HOME_LAUNCH_POLISH.md")

check(
    "Home next step is computed from workflow state",
    "const nextStep = !dataset.columns.length" in home and "Recommended next step" in home and "home-next-step" in styles,
    "Home computes and displays a workflow-aware next-step launcher.",
)
check(
    "Home uses compact workflow status instead of sparse step cards",
    "home-workflow-list" in home and "workflow-cards" not in home and ".home-workflow-list" in styles,
    "Home step status is rendered as a compact list.",
)
check(
    "Home keeps project actions explicit",
    "Save project" in home and "Open project" in home and "Continue recent project" in home,
    "Home keeps save/open/recent/demo project entry points visible.",
)
check(
    "Data supports a clear model bridge",
    "Open Model Designer" in data and "Create Constructs From Prefixes" in data and "prefixGroups" in data,
    "Data workspace can move users from imported variables into SEM model creation.",
)
check(
    "Sample/developer data details are not expanded by default",
    "showValidationDetails, setShowValidationDetails" in data and "Sample dataset details" in data,
    "Data keeps bundled sample details in a collapsible disclosure instead of the main import path.",
)
check(
    "Registry and docs are wired",
    "v1_6_2_data_home_launch_polish" in registry and "v1.6.2 Data/Home launch polish" in docs,
    "Development slice registry and milestone documentation reference v1.6.2.",
)
check(
    "No normal launch copy contains mojibake",
    "RÃ‚" not in home + data and "Ã‚" not in home + data and "Ã¢" not in home + data,
    "Home and Data sources contain no visible mojibake sequences.",
)

passed = all(item["passed"] for item in checks)
result = {
    "passed": passed,
    "milestone": "v1_6_2_data_home_launch_polish",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "checks": checks,
}

out = ROOT / "validation" / "results" / "v162_data_home_audit.json"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(result, indent=2), encoding="utf-8")

if not passed:
    for item in checks:
        if not item["passed"]:
            print(f"FAILED: {item['name']} - {item['evidence']}")
    raise SystemExit(1)

print("v1.6.2 data/home audit passed")
