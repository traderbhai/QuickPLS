import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
MILESTONE = "v1_5_9_report_publication_workflow_redesign"


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    failures: list[str] = []
    smoke_path = RESULTS / "v159_report_publication_smoke.json"
    if not smoke_path.exists():
        failures.append("missing v159_report_publication_smoke.json")
        smoke = {}
    else:
        smoke = read_json(smoke_path)
        if not smoke.get("passed"):
            failures.append("v1.5.9 report publication smoke did not pass")

    registry = read_json(ROOT / "validation" / "development_slices.json")
    if registry.get("current_stage") != MILESTONE:
        failures.append("registry current_stage is not v1.5.9")
    matching = [item for item in registry.get("slices", []) if item.get("id") == MILESTONE]
    if not matching:
        failures.append("v1.5.9 registry slice is missing")
    else:
        slice_ = matching[0]
        if slice_.get("status") != "validated" or not slice_.get("stable_output"):
            failures.append("v1.5.9 registry slice is not validated/stable")
        open_gates = [gate for gate in slice_.get("gates", []) if gate.get("status") != "passed"]
        if open_gates:
            failures.append("v1.5.9 registry slice has non-passed gates")

    docs_path = ROOT / "docs" / "V1_5_9_REPORT_PUBLICATION_WORKFLOW_REDESIGN.md"
    if not docs_path.exists():
        failures.append("v1.5.9 milestone doc is missing")
    else:
        text = docs_path.read_text(encoding="utf-8").lower()
        for phrase in [
            "frontend-only",
            "publication preview",
            "export actions",
            "run comparison",
            "numerical fingerprints",
        ]:
            if phrase not in text:
                failures.append(f"v1.5.9 doc missing phrase: {phrase}")

    package = read_json(ROOT / "package.json")
    scripts = package.get("scripts", {})
    required_scripts = [
        "qpls:v159:report-smoke",
        "qpls:v159:report-audit",
        "qpls:v159:report-publication",
    ]
    for script in required_scripts:
        if script not in scripts:
            failures.append(f"package.json missing script {script}")

    roadmap = (ROOT / "crates" / "qpls-core" / "src" / "roadmap.rs").read_text(encoding="utf-8")
    if MILESTONE not in roadmap:
        failures.append("roadmap test does not reference v1.5.9")

    payload = {
        "passed": not failures,
        "milestone": MILESTONE,
        "checks": {
            "smoke_passed": bool(smoke.get("passed")),
            "registry_current_stage": registry.get("current_stage"),
            "docs_present": docs_path.exists(),
            "package_scripts_present": all(script in scripts for script in required_scripts),
            "roadmap_references_milestone": MILESTONE in roadmap,
        },
        "failures": failures,
        "boundary": "frontend-only report publication workflow redesign",
    }
    RESULTS.mkdir(parents=True, exist_ok=True)
    (RESULTS / "v159_report_publication_audit.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    if failures:
        print(json.dumps(payload, indent=2))
        return 1
    print("v1.5.9 report publication audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
