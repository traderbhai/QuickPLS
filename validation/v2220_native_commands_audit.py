import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "v2220_native_commands_audit.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


topbar = read("src/components/TopBar.tsx")
types = read("src/types.ts")
registry = read("src/domain/desktopCommands.ts")
package = read("package.json")
slices = read("validation/development_slices.json")

required_menus = ["file", "edit", "data", "model", "calculate", "results", "report", "view", "tools", "window", "help"]
required_dialogs = [
    "new_project",
    "open_project",
    "import_data",
    "export_options",
    "calculation_setup",
    "method_scope",
    "settings",
    "help_shortcuts",
]
checks = []
for menu in required_menus:
    checks.append((f"menu id {menu} typed", f'"{menu}"' in types))
    checks.append((f"menu id {menu} registered", f'id: "{menu}"' in registry))
for dialog in required_dialogs:
    checks.append((f"dialog {dialog} typed", f'"{dialog}"' in types))
checks.extend([
    ("command definitions exist", "DESKTOP_COMMANDS" in registry),
    ("shortcuts captured", "Ctrl+S" in registry and "F5" in registry),
    ("disabled reason contract captured", "requiresReasonWhenDisabled" in registry),
    ("TopBar uses menu order", "DESKTOP_MENU_ORDER" in topbar),
    ("Tools menu rendered", 'id: "tools"' in topbar),
    ("Window menu rendered", 'id: "window"' in topbar),
    ("command status setter wired", "setDesktopCommandStatus" in topbar),
    ("v2.22 script exists", "qpls:v2220:native-commands" in package),
    ("v2.22 gate registered", '"id": "v2_22_0_menu_commands_dialogs_native_base"' in slices),
])

issues = [{"check": name, "status": "failed"} for name, ok in checks if not ok]
payload = {
    "milestone": "v2_22_0_menu_commands_dialogs_native_base",
    "passed": not issues,
    "checks": [{"check": name, "status": "passed" if ok else "failed"} for name, ok in checks],
    "issues": issues,
    "frontend_only": True,
}
RESULT.parent.mkdir(parents=True, exist_ok=True)
RESULT.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
if issues:
    raise SystemExit(f"v2.22 native commands audit failed: {issues}")
print(f"v2.22 native commands audit passed: {RESULT}")
