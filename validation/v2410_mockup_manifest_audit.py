import json
from datetime import UTC, datetime
from pathlib import Path

from lib.v2_ui_audit import RESULTS, write_result

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "validation" / "mockups" / "v2410_mockup_manifest.json"
OUTPUT = "v2410_mockup_manifest_audit.json"
MILESTONE = "v2_41_0_full_mockup_screen_parity_pass"


def main() -> int:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    mockup_dir = Path(manifest["mockup_dir"])
    source_pngs = sorted(path.name for path in mockup_dir.glob("*.png"))
    entries = manifest.get("states", [])
    mapped = [entry.get("file") for entry in entries]
    missing_files = sorted(set(source_pngs) - set(mapped))
    orphaned_files = sorted(set(mapped) - set(source_pngs))
    duplicate_files = sorted({name for name in mapped if mapped.count(name) > 1})
    ids = [entry.get("id") for entry in entries]
    duplicate_ids = sorted({name for name in ids if ids.count(name) > 1})
    dialogs = {entry.get("dialog") for entry in entries if entry.get("kind") == "dialog"}
    checks = {
        "manifest_exists": MANIFEST.exists(),
        "mockup_dir_exists": mockup_dir.exists(),
        "source_png_count_expected": len(source_pngs) == 19,
        "all_source_pngs_mapped": not missing_files,
        "no_orphaned_manifest_files": not orphaned_files,
        "no_duplicate_mapping_files": not duplicate_files,
        "no_duplicate_state_ids": not duplicate_ids,
        "main_screens_mapped": {"home", "data", "model", "setup", "run", "results", "report", "trust", "settings"}.issubset({entry.get("view") for entry in entries}),
        "required_dialogs_mapped": {"new_project", "import_data", "calculation_setup", "method_scope", "export_options"}.issubset(dialogs),
        "required_text_contracts_present": all(entry.get("required") for entry in entries),
    }
    issues = [
        {"id": key, "severity": "high", "detail": f"Failed check: {key}"}
        for key, passed in checks.items()
        if not passed
    ]
    payload = {
        "passed": not issues,
        "milestone": MILESTONE,
        "generatedAt": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "checks": checks,
        "issues": issues,
        "source_pngs": source_pngs,
        "mapped_files": mapped,
        "missing_files": missing_files,
        "orphaned_files": orphaned_files,
        "duplicate_files": duplicate_files,
        "duplicate_ids": duplicate_ids,
    }
    write_result(OUTPUT, payload)
    print(json.dumps(payload, indent=2))
    return 0 if payload["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
