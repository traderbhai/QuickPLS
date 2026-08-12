import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def main() -> int:
    package = json.loads(read_text("package.json"))
    registry = json.loads(read_text("validation/development_slices.json"))
    cargo = read_text("Cargo.toml")
    tauri = json.loads(read_text("src-tauri/tauri.conf.json"))
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    smoke_path = RESULTS / "v200_shell_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2 shell": registry.get("current_stage") == "v2_0_0_design_system_and_shell",
        "registry contains v2 shell slice": any(s.get("id") == "v2_0_0_design_system_and_shell" for s in registry.get("slices", [])),
        "package version is 2.0.0": package.get("version") == "2.0.0",
        "cargo version is 2.0.0": 'version = "2.0.0"' in cargo,
        "tauri version is 2.0.0": tauri.get("version") == "2.0.0",
        "roadmap expects v2 shell": "v2_0_0_design_system_and_shell" in roadmap,
        "release artifact label is v2 shell": "v2_0_0_design_system_and_shell" in package["scripts"].get("qpls:release:artifacts", ""),
        "no stale visible v1.8.1 shell mark": "v1.8.1 method applicability guidance" not in read_text("src/components/TopBar.tsx"),
    }
    failed = [name for name, passed in checks.items() if not passed]
    report = {
        "milestone": "v2_0_0_design_system_and_shell",
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v200_shell_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report, indent=2))
        return 1
    print("v2.0.0 shell audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
