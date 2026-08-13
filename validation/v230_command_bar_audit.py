import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8-sig")


def read_json(relative: str):
    return json.loads(read_text(relative))


def main() -> int:
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    styles = read_text("src/styles.css")
    docs = read_text("docs/V2_3_0_GLOBAL_COMMAND_BAR_READINESS.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v230_command_bar_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_3_0_global_command_bar_readiness"
    quickpls_packages = [
        "qpls-assessment",
        "qpls-cli",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
        "quickpls-desktop",
    ]
    source_bundle = "\n".join([topbar, styles])
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    forbidden_mojibake = ["RÃƒÆ’Ã†â€™", "RÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²", "ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢", "ÃƒÆ’Ã¢â‚¬Å¡ "]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.3.0": registry.get("current_stage") == target_stage,
        "registry contains v2.3.0 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.3.0": package.get("version") == "2.3.0",
        "package lock root version is 2.3.0": package_lock.get("version") == "2.3.0" and package_lock.get("packages", {}).get("", {}).get("version") == "2.3.0",
        "cargo workspace version is 2.3.0": 'version = "2.3.0"' in cargo,
        "quickpls lock versions are 2.3.0": all(f'name = "{name}"\nversion = "2.3.0"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.3.0": tauri.get("version") == "2.3.0",
        "roadmap expects v2.3.0": target_stage in roadmap,
        "topbar shows v2.3.0 command bar readiness": "v2.3.0 command bar readiness" in topbar,
        "artifact label is v2.3.0": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.3.0 scripts": all(key in package["scripts"] for key in [
            "qpls:v230:command-bar-smoke",
            "qpls:v230:command-bar-audit",
            "qpls:v230:command-bar",
        ]),
        "command bar exposes run metadata": all(token in topbar for token in [
            "command-run-cluster",
            "data-command-bar-state",
            "data-run-state",
            "data-run-method",
            "data-run-blocker-id",
            "data-run-blocker-view",
            "data-run-disabled-reason",
            "data-run-blocker-action",
        ]),
        "run disabled reason is nearby and accessible": "aria-describedby={!activeJob && !canRun ? \"run-disabled-reason\" : undefined}" in topbar and "aria-label={runDisabledLabel}" in topbar and "title={topBlocker?.detail ?? readiness.summary}" in topbar,
        "blocker navigation uses destination context": "setView(runBlockerTarget, {" in topbar and "coachId: \"top-command-bar\"" in topbar and "actionLabel: `Run blocker: ${topBlocker.label}`" in topbar,
        "old direct store blocker navigation removed": "useWorkspace.getState().setView(topBlocker.actionView)" not in topbar,
        "command blocker styling supports structured chip": ".command-run-cluster" in styles and ".command-blocker-chip strong" in styles and ".command-blocker-chip small" in styles,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "No numerical fingerprint changes." in docs,
        "delivery status updated": target_stage in delivery and "v2.3.0 Global Command Bar Readiness" in delivery,
        "development ledger updated": target_stage in ledger and "v2.3.0 Global Command Bar Readiness" in ledger,
        "target source has no SmartPLS equivalence claim": not any(phrase in source_bundle.lower() for phrase in forbidden_equivalence),
        "target source has no mojibake": not any(token in source_bundle for token in forbidden_mojibake),
    }

    failed = [name for name, passed in checks.items() if not passed]
    payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v230_command_bar_audit.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.3.0 global command bar readiness audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
