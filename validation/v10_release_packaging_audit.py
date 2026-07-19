"""Final v1.0 release packaging audit."""

import glob
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v10_release_packaging_audit.json"
VERSION = "1.0.0"


def read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def mtime(path: Path) -> float:
    return path.stat().st_mtime if path.exists() else 0


def file_status(relative: str) -> dict:
    path = ROOT / relative
    return {
        "path": relative,
        "present": path.exists(),
        "bytes": path.stat().st_size if path.exists() else 0,
    }


def package_lock_version_ok() -> bool:
    value = read_json(ROOT / "package-lock.json")
    root_pkg = value.get("packages", {}).get("", {})
    return value.get("version") == VERSION and root_pkg.get("version") == VERSION


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    package = read_json(ROOT / "package.json")
    tauri = read_json(ROOT / "src-tauri" / "tauri.conf.json")
    cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    dist_index = ROOT / "dist" / "index.html"
    release_exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    nsis = [Path(path) for path in glob.glob(str(ROOT / "target" / "release" / "bundle" / "nsis" / f"QuickPLS_{VERSION}_x64-setup.exe"))]
    desktop_smoke = read_json(RESULTS / "v10_desktop_smoke_check.json")
    docs = [
        "docs/RELEASE_NOTES_V1_0.md",
        "docs/INSTALLATION_V1_0.md",
        "docs/DEPENDENCY_NOTICES_V1_0.md",
        "docs/V1_SUPPORTED_SCOPE.md",
        "docs/VALIDATION_ARTIFACT_INDEX_V1_0.md",
    ]
    install_text = (ROOT / "docs" / "INSTALLATION_V1_0.md").read_text(encoding="utf-8") if (ROOT / "docs" / "INSTALLATION_V1_0.md").exists() else ""
    checks = {
        "package_json_version": package.get("version") == VERSION,
        "package_lock_version": package_lock_version_ok(),
        "cargo_workspace_version": f'version = "{VERSION}"' in cargo,
        "tauri_version": tauri.get("version") == VERSION,
        "frontend_build_exists": dist_index.exists(),
        "release_executable_exists": release_exe.exists(),
        "release_executable_newer_than_frontend": release_exe.exists() and dist_index.exists() and mtime(release_exe) >= mtime(dist_index),
        "nsis_installer_exists": bool(nsis),
        "offline_launch_smoke_passed": desktop_smoke.get("passed") is True,
        "docs_and_notices_present": all((ROOT / doc).exists() and (ROOT / doc).stat().st_size > 0 for doc in docs),
        "signing_status_explicit": "unsigned" in install_text.lower() and "code-signing certificate" in install_text.lower(),
        "no_dev_server_runtime_dependency": '"frontendDist": "../dist"' in (ROOT / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.0.0 release packaging audit",
        "passed": all(checks.values()),
        "version": VERSION,
        "checks": checks,
        "release_executable": file_status("target/release/quickpls-desktop.exe"),
        "nsis_installers": [str(path.relative_to(ROOT)) for path in nsis],
        "docs": [file_status(doc) for doc in docs],
        "note": "The v1.0 installer is unsigned unless a certificate is provided and a separate signing audit is added.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
