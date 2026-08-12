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
    styles = read_text("src/styles.css")
    docs = read_text("docs/V2_0_3_VISUAL_FIDELITY_FOUNDATION.md")
    contract = read_text("docs/V2_UI_VISUAL_CONTRACT.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v203_visual_fidelity_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    required_slice = next((s for s in registry.get("slices", []) if s.get("id") == "v2_0_3_visual_fidelity_foundation"), None)
    required_css = [
        "--q2-page-gutter-x",
        "--q2-panel-pad",
        "--q2-section-gap",
        "--q2-control-height",
        "--q2-shadow-soft",
        ".qpls2-panel",
        ".qpls2-chip",
        ".qpls2-primary-action",
        ".results-v2-run-hero",
        ".results-v2-table-shell",
    ]
    boundary_text = "\n".join([docs, contract, delivery, ledger])

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.0.3": registry.get("current_stage") == "v2_0_3_visual_fidelity_foundation",
        "registry contains v2.0.3 slice": required_slice is not None,
        "v2.0.3 gates are passed": bool(required_slice) and all(g.get("status") == "passed" for g in required_slice.get("gates", [])),
        "package version is 2.0.3": package.get("version") == "2.0.3",
        "package lock version is 2.0.3": '"version": "2.0.3"' in read_text("package-lock.json"),
        "cargo version is 2.0.3": 'version = "2.0.3"' in cargo,
        "tauri version is 2.0.3": tauri.get("version") == "2.0.3",
        "roadmap expects v2.0.3": "v2_0_3_visual_fidelity_foundation" in roadmap,
        "release artifact label is v2.0.3": "v2_0_3_visual_fidelity_foundation" in package["scripts"].get("qpls:release:artifacts", ""),
        "docs and contract are present": "QuickPLS 2.0 Visual Contract" in contract and "v2.0.3 Visual Fidelity Foundation" in docs,
        "CSS visual foundation exists": all(token in styles for token in required_css),
        "docs record frontend-only boundary": "no statistical engine" in boundary_text.lower() or "does not change statistical engines" in boundary_text.lower(),
        "delivery and ledger mention v2.0.3": "v2.0.3 Visual Fidelity Foundation" in delivery and "v2.0.3 Visual Fidelity Foundation" in ledger,
        "no mojibake in v2 docs/styles": "RÃ" not in "\n".join([styles, docs, contract]),
    }
    failed = [name for name, passed in checks.items() if not passed]
    report = {
        "milestone": "v2_0_3_visual_fidelity_foundation",
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v203_visual_fidelity_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report, indent=2))
        return 1
    print("v2.0.3 visual fidelity audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
