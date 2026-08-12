import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "v2210_native_shell_audit.json"


def contains(path: str, text: str) -> bool:
    return text in (ROOT / path).read_text(encoding="utf-8")


checks = [
    ("package version is 2.22.0", contains("package.json", '"version": "2.22.0"')),
    ("tauri version is 2.22.0", contains("src-tauri/tauri.conf.json", '"version": "2.22.0"')),
    ("cargo workspace version is 2.22.0", contains("Cargo.toml", 'version = "2.22.0"')),
    ("native shell marker exists", contains("src/components/TopBar.tsx", "data-v221-native-shell")),
    ("native command strip marker exists", contains("src/components/TopBar.tsx", "data-v222-command-feedback")),
    ("bottom command feedback rendered", contains("src/components/StatusBar.tsx", "status-command-feedback")),
    ("status feedback styles exist", contains("src/styles.css", ".status-command-feedback")),
    ("registry current stage updated", contains("validation/development_slices.json", '"current_stage": "v2_22_0_menu_commands_dialogs_native_base"')),
    ("v2.21 gate registered", contains("validation/development_slices.json", '"id": "v2_21_0_desktop_design_system_shell"')),
]

issues = [{"check": name, "status": "failed"} for name, ok in checks if not ok]
payload = {
    "milestone": "v2_21_0_desktop_design_system_shell",
    "passed": not issues,
    "checks": [{"check": name, "status": "passed" if ok else "failed"} for name, ok in checks],
    "issues": issues,
    "frontend_only": True,
}
RESULT.parent.mkdir(parents=True, exist_ok=True)
RESULT.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
if issues:
    raise SystemExit(f"v2.21 native shell audit failed: {issues}")
print(f"v2.21 native shell audit passed: {RESULT}")
