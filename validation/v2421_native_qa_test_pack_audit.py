from pathlib import Path

from lib.v2_ui_audit import ROOT, RESULTS, read_json, read_text, write_result

TARGET = "v2_42_1_native_shell_qa_test_pack"
VERSION = "2.42.1"

checks = []
failures = []


def add(name: str, passed: bool, evidence: str):
    checks.append({"name": name, "passed": passed, "evidence": evidence})
    if not passed:
        failures.append({"name": name, "evidence": evidence})


screen = read_json("validation/results/v2421_native_screen_qa_smoke.json")
interaction = read_json("validation/results/v2421_native_interaction_wiring_smoke.json")
trace = read_json("validation/results/v2421_native_web_trace_audit.json")

add("screen QA smoke passed", bool(screen.get("passed")), "validation/results/v2421_native_screen_qa_smoke.json")
add("interaction wiring smoke passed", bool(interaction.get("passed")), "validation/results/v2421_native_interaction_wiring_smoke.json")
add("old-shell trace audit passed", bool(trace.get("passed")), "validation/results/v2421_native_web_trace_audit.json")

package = read_text("package.json")
for script in [
    "qpls:v2421:screen-qa",
    "qpls:v2421:interaction-wiring",
    "qpls:v2421:web-trace-audit",
    "qpls:v2421:qa-test-pack",
]:
    add(f"npm script {script} exists", f'"{script}"' in package, "package.json")

for doc in [
    "docs/V2_42_1_NATIVE_SHELL_QA_TEST_PACK.md",
    "docs/V2_ACTIVE_MILESTONE.md",
    "docs/DELIVERY_STATUS.md",
    "docs/DEVELOPMENT_LEDGER.md",
]:
    add(f"{doc} exists", (ROOT / doc).exists(), doc)

registry = read_json("validation/development_slices.json")
target_slice = next((item for item in registry.get("slices", []) if item.get("id") == TARGET), None)
add("registry current stage is v2.42.1", registry.get("current_stage") == TARGET, "validation/development_slices.json")
add("registry slice exists", target_slice is not None, "validation/development_slices.json")
add("registry slice validated", bool(target_slice and target_slice.get("status") == "validated"), "validation/development_slices.json")

screenshot_dir = RESULTS / "screens" / "v2421" / "native-qa"
expected_screenshots = [
    "home.png",
    "data.png",
    "model.png",
    "setup.png",
    "run.png",
    "results.png",
    "report.png",
    "trust.png",
    "settings.png",
    "dialog-import_data.png",
    "dialog-calculation_setup.png",
]
for filename in expected_screenshots:
    add(f"screenshot {filename} exists", (screenshot_dir / filename).exists(), str(screenshot_dir / filename))

add("release artifact label is v2.42.1", "v2_42_1_native_shell_qa_test_pack" in package, "package.json")
add("package version is 2.42.1", '"version": "2.42.1"' in package, "package.json")

result = {
    "passed": not failures,
    "target": TARGET,
    "version": VERSION,
    "checks": checks,
    "failures": failures,
}
write_result("v2421_native_qa_test_pack_audit.json", result)
if failures:
    raise SystemExit(1)
