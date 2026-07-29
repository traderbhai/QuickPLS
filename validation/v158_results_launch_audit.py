import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
MILESTONE = "v1_5_8_results_workspace_launch_redesign"


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    failures: list[str] = []
    smoke_path = RESULTS / "v158_results_launch_smoke.json"
    if not smoke_path.exists():
        failures.append("missing v158_results_launch_smoke.json")
        smoke = {}
    else:
        smoke = read_json(smoke_path)
        if not smoke.get("passed"):
            failures.append("v158 results launch smoke did not pass")

    registry = read_json(ROOT / "validation" / "development_slices.json")
    if registry.get("current_stage") != MILESTONE:
        failures.append("registry current_stage is not v1.5.8")
    matching = [item for item in registry.get("slices", []) if item.get("id") == MILESTONE]
    if not matching:
        failures.append("v1.5.8 registry slice is missing")
    else:
        slice_ = matching[0]
        if slice_.get("status") != "validated" or not slice_.get("stable_output"):
            failures.append("v1.5.8 registry slice is not validated/stable")
        open_gates = [gate for gate in slice_.get("gates", []) if gate.get("status") != "passed"]
        if open_gates:
            failures.append("v1.5.8 registry slice has non-passed gates")

    docs_path = ROOT / "docs" / "V1_5_8_RESULTS_WORKSPACE_LAUNCH_REDESIGN.md"
    if not docs_path.exists():
        failures.append("v1.5.8 milestone doc is missing")
    else:
        text = docs_path.read_text(encoding="utf-8")
        text_lower = text.lower()
        for phrase in ["frontend-only", "Result", "HTMT", "Mediation", "numerical fingerprints"]:
            if phrase.lower() not in text_lower:
                failures.append(f"v1.5.8 doc missing phrase: {phrase}")

    package = read_json(ROOT / "package.json")
    scripts = package.get("scripts", {})
    for script in [
        "qpls:v158:results-launch-smoke",
        "qpls:v158:results-launch-audit",
        "qpls:v158:results-launch",
    ]:
        if script not in scripts:
            failures.append(f"package.json missing script {script}")

    roadmap = (ROOT / "crates" / "qpls-core" / "src" / "roadmap.rs").read_text(encoding="utf-8")
    if MILESTONE not in roadmap:
        failures.append("roadmap test does not reference v1.5.8")

    payload = {
        "passed": not failures,
        "milestone": MILESTONE,
        "checks": {
            "smoke_passed": bool(smoke.get("passed")),
            "registry_current_stage": registry.get("current_stage"),
            "docs_present": docs_path.exists(),
            "package_scripts_present": all(
                script in scripts
                for script in [
                    "qpls:v158:results-launch-smoke",
                    "qpls:v158:results-launch-audit",
                    "qpls:v158:results-launch",
                ]
            ),
            "roadmap_references_milestone": MILESTONE in roadmap,
        },
        "failures": failures,
        "boundary": "frontend-only results workspace launch redesign",
    }
    RESULTS.mkdir(parents=True, exist_ok=True)
    (RESULTS / "v158_results_launch_audit.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    if failures:
        print(json.dumps(payload, indent=2))
        return 1
    print("v1.5.8 results launch audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
