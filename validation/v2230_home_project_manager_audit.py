import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "v2230_home_project_manager_audit.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def main() -> int:
    package = json.loads(read("package.json"))
    slices = json.loads(read("validation/development_slices.json"))
    home = read("src/components/OnboardingWorkspace.tsx")
    styles = read("src/styles.css")
    roadmap = read("crates/qpls-core/src/roadmap.rs")

    checks = {
        "version is 2.23.0": package.get("version") == "2.23.0",
        "current stage is v2.23": slices.get("current_stage") == "v2_23_0_home_project_manager",
        "roadmap expects v2.23": "v2_23_0_home_project_manager" in roadmap,
        "home is marked as project manager": "data-v223-project-manager" in home,
        "recent/recovery/project summary surfaces exist": all(token in home for token in ["Recent projects", "Recovery and autosave", "Project summary", "Quick links"]),
        "desktop list styling exists": all(token in styles for token in [".home-v223-manager-grid", ".home-v223-recent-row", ".home-v223-link-list"]),
        "no backend numerical references": not any(token in home for token in ["F_ml", "qpls-estimation", "numerical fingerprint"]),
        "no SmartPLS equivalence claim": "equivalent to SmartPLS" not in home,
        "package scripts exist": all(script in package.get("scripts", {}) for script in ["qpls:v2230:home-smoke", "qpls:v2230:home-audit", "qpls:v2230:home-project-manager"]),
    }

    result = {
        "passed": all(checks.values()),
        "milestone": "v2_23_0_home_project_manager",
        "checks": checks,
        "failed": [name for name, passed in checks.items() if not passed],
    }
    RESULT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    if not result["passed"]:
        print(json.dumps(result, indent=2))
        return 1
    print(f"v2.23 home project manager audit passed: {RESULT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
