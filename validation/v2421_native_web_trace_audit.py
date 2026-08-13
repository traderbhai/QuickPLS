from pathlib import Path

from lib.v2_ui_audit import (
    ROOT,
    RESULTS,
    FORBIDDEN_MOJIBAKE,
    FORBIDDEN_SMARTPLS_EQUIVALENCE,
    frontend_boundary_check,
    read_json,
    read_text,
    shared_v2_metadata_checks,
    source_bundle,
    write_result,
)

TARGET = "v2_42_1_native_shell_qa_test_pack"
VERSION = "2.42.1"
LABEL = "v2_42_1_native_shell_qa_test_pack"

checks = []
failures = []


def add(name: str, passed: bool, evidence: str):
    checks.append({"name": name, "passed": passed, "evidence": evidence})
    if not passed:
        failures.append({"name": name, "evidence": evidence})


metadata = shared_v2_metadata_checks(
    version=VERSION,
    target_stage=TARGET,
    expected_label="v2.42.1 native shell QA test pack",
    package=read_json("package.json"),
    package_lock=read_json("package-lock.json"),
    registry=read_json("validation/development_slices.json"),
    cargo=read_text("Cargo.toml"),
    cargo_lock=read_text("Cargo.lock"),
    tauri=read_json("src-tauri/tauri.conf.json"),
    roadmap=read_text("crates/qpls-core/src/roadmap.rs"),
    topbar=read_text("src/components/TopBar.tsx"),
)
for name, passed in metadata.items():
    add(name, bool(passed), "release metadata and registry files")

app_source = read_text("src/App.tsx")
native_source = read_text("src/v2/NativePrototypeApp.tsx")
css_source = read_text("src/v2/nativePrototype.css")
bundle = source_bundle([
    "src/App.tsx",
    "src/v2/NativePrototypeApp.tsx",
    "src/v2/nativePrototype.css",
    "src/components/TopBar.tsx",
    "docs/V2_ACTIVE_MILESTONE.md",
])

add(
    "default route mounts native candidate shell",
    "return <NativeShellCandidateApp />" in app_source and "legacy_shell" in app_source,
    "src/App.tsx keeps explicit legacy_shell fallback and defaults to NativeShellCandidateApp.",
)
add(
    "legacy shell is explicit opt-in only",
    app_source.count("legacy_shell") >= 1 and "?legacy_shell=1" not in app_source,
    "Legacy shell is gated by query parsing and is not the default route.",
)
add(
    "native parity markers remain present",
    "data-v241-mockup-parity" in native_source and "data-v239-shell-mode" in native_source,
    "Native shell exposes parity and shell mode markers for QA.",
)
add(
    "dialog escape handler present",
    'event.key === "Escape"' in native_source and "window.addEventListener(\"keydown\"" in native_source,
    "Dialog manager closes task dialogs with Escape.",
)
add(
    "extra feature backlog exists",
    (ROOT / "docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md").exists(),
    "docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md records deferred non-mockup extras.",
)

for forbidden in FORBIDDEN_MOJIBAKE:
    add(f"no mojibake token {forbidden!r}", forbidden not in bundle, "Source bundle is free of mojibake.")
for forbidden in FORBIDDEN_SMARTPLS_EQUIVALENCE:
    add(f"no SmartPLS equivalence claim {forbidden!r}", forbidden.lower() not in bundle.lower(), "Source bundle avoids equivalence claims.")

legacy_phrases = ["v1.5", "v1.4", "v1.0 stable scope", "dashboard"]
for phrase in legacy_phrases:
    add(f"default shell source avoids stale phrase {phrase!r}", phrase.lower() not in (native_source + css_source).lower(), "Native shell source is clear of stale legacy wording.")

screen_smoke = read_json("validation/results/v2421_native_screen_qa_smoke.json")
interaction = read_json("validation/results/v2421_native_interaction_wiring_smoke.json")
add("screen QA smoke passed", bool(screen_smoke.get("passed")), "validation/results/v2421_native_screen_qa_smoke.json")
add("interaction wiring smoke passed", bool(interaction.get("passed")), "validation/results/v2421_native_interaction_wiring_smoke.json")

add(
    "frontend-only boundary",
    frontend_boundary_check("v2.42.1 native QA test pack audit"),
    "Audit source does not include estimator, assessment, or resampling crate paths.",
)

result = {
    "passed": not failures,
    "target": TARGET,
    "version": VERSION,
    "checks": checks,
    "failures": failures,
}
write_result("v2421_native_web_trace_audit.json", result)
if failures:
    raise SystemExit(1)
