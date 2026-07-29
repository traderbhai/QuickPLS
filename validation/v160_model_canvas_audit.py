import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


checks = []


def check(name: str, passed: bool, evidence: str) -> None:
    checks.append({"name": name, "passed": bool(passed), "evidence": evidence})


app = read("src/App.tsx")
store = read("src/store.ts")
canvas = read("src/components/ModelCanvas.tsx")
explorer = read("src/components/Explorer.tsx")
inspector = read("src/components/Inspector.tsx")
styles = read("src/styles.css")
combined = "\n".join([canvas, explorer, inspector])

check(
    "Right inspector collapse is wired",
    "inspectorCollapsed" in app and "setInspectorCollapsed" in store and "inspector-collapsed" in styles,
    "App shell, store, and CSS include inspector-collapsed support.",
)
check(
    "Canvas View menu controls both side panels",
    "Collapse left explorer" in canvas and "Collapse right inspector" in canvas,
    "ModelCanvas View menu exposes left explorer and right inspector collapse actions.",
)
check(
    "Minimap is opt-in",
    "const [showMiniMap, setShowMiniMap] = useState(false)" in canvas
    and "Show minimap" in canvas
    and "showMiniMap && !resultDiagramMode" in canvas,
    "Minimap defaults off and appears only when explicitly enabled.",
)
check(
    "Overlay status is compact",
    "canvas-overlay-status compact" in canvas and ".canvas-overlay-status.compact" in styles,
    "Overlay status has a compact class and compact CSS placement.",
)
check(
    "Context toolbar secondary actions are grouped",
    "context-menu-lite" in canvas and "Route" in canvas and "Indicators" in canvas,
    "Path routing and indicator side commands are grouped instead of always occupying toolbar width.",
)
check(
    "Explorer construct cards are less dense",
    "explorer-card-more" in explorer and "Reset indicators" in explorer and " - {node.data.indicators.length} indicators - " in explorer,
    "Low-frequency construct card commands live under More and mojibake separators were removed.",
)
check(
    "Inspector can collapse and is no longer the primary layout surface",
    "inspector-collapse-button" in inspector and "<details className=\"inspector-section\"><summary>Indicators</summary>" in inspector,
    "Inspector exposes collapse control and keeps nonessential sections collapsed by default.",
)
check(
    "No visible mojibake remains in model shell sources",
    "RÂ" not in combined and "Â·" not in combined and "â†’" not in combined,
    "ModelCanvas, Explorer, and Inspector contain no R²/separator/arrow mojibake.",
)

passed = all(item["passed"] for item in checks)
result = {
    "passed": passed,
    "milestone": "v1_6_0_model_canvas_shell_and_panel_polish",
    "generated_at": __import__("datetime").datetime.utcnow().isoformat() + "Z",
    "checks": checks,
}

out = ROOT / "validation" / "results" / "v160_model_canvas_audit.json"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(result, indent=2), encoding="utf-8")

if not passed:
    for item in checks:
        if not item["passed"]:
            print(f"FAILED: {item['name']} - {item['evidence']}")
    raise SystemExit(1)

print("v1.6.0 model canvas audit passed")
