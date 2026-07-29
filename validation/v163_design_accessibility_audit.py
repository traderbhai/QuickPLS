import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


checks = []


def check(name: str, passed: bool, evidence: str) -> None:
    checks.append({"name": name, "passed": bool(passed), "evidence": evidence})


topbar = read("src/components/TopBar.tsx")
styles = read("src/styles.css")
tests = read("src/components/accessibilityContracts.test.ts")
registry = read("validation/development_slices.json")
docs = read("docs/V1_6_3_GLOBAL_DESIGN_ACCESSIBILITY_PASS.md")
package = read("package.json")

source_paths = [
    "src/components/TopBar.tsx",
    "src/components/OnboardingWorkspace.tsx",
    "src/components/DataWorkspace.tsx",
    "src/components/ModelCanvas.tsx",
    "src/components/AnalysisCatalog.tsx",
    "src/components/RunWorkspace.tsx",
    "src/components/RunHistory.tsx",
    "src/components/ReportsWorkspace.tsx",
    "src/styles.css",
]
combined_source = "\n".join(read(path) for path in source_paths)
forbidden_mojibake = ["RÂ", "Â²", "Ã", "â€", "ï¿½"]

check(
    "Visible milestone label is current",
    "v1.6.3 design and accessibility pass" in topbar and "v1.5.3 layout, copy, and readiness polish" not in topbar,
    "TopBar must not display stale release milestone text.",
)
check(
    "Global focus-visible treatment exists",
    "button:focus-visible" in styles and "input:focus-visible" in styles and "select:focus-visible" in styles,
    "Core desktop controls keep a visible keyboard focus outline.",
)
check(
    "Table and canvas accessibility contracts are tested",
    "keeps large table surfaces keyboard-focusable and named" in tests
    and "keeps SEM canvas overlay state visible to users" in tests
    and "persistent desktop readiness checklist" in tests,
    "Existing Vitest accessibility contracts cover tables, canvas overlay status, shortcuts, and readiness.",
)
check(
    "Top Run disabled reason is programmatically linked",
    'aria-describedby={!activeJob && !canRun ? "run-disabled-reason" : undefined}' in topbar
    and 'id="run-disabled-reason"' in topbar
    and "command-blocker-chip" in topbar,
    "The main Run button keeps a nearby accessible disabled reason.",
)
check(
    "Normal UI source has no mojibake markers",
    not any(marker in combined_source for marker in forbidden_mojibake),
    "Normal user-facing source must not contain mojibake such as RÂ² or replacement characters.",
)
check(
    "Status language remains scoped",
    "Validated for documented QuickPLS scope" in combined_source
    and "SmartPLS equivalence" not in combined_source
    and "identical to SmartPLS" not in combined_source,
    "Status wording stays scoped and avoids competitor-equivalence claims.",
)
check(
    "Registry, package scripts, and docs are wired",
    "v1_6_3_global_design_system_and_accessibility_pass" in registry
    and "qpls:v163:design-accessibility" in package
    and "v1.6.3 Global design-system and accessibility pass" in docs,
    "Development slice registry, npm scripts, and milestone docs reference v1.6.3.",
)

passed = all(item["passed"] for item in checks)
result = {
    "passed": passed,
    "milestone": "v1_6_3_global_design_system_and_accessibility_pass",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "checks": checks,
}

out = ROOT / "validation" / "results" / "v163_design_accessibility_audit.json"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(result, indent=2), encoding="utf-8")

if not passed:
    for item in checks:
        if not item["passed"]:
            print(f"FAILED: {item['name']} - {item['evidence']}")
    raise SystemExit(1)

print("v1.6.3 design/accessibility audit passed")
