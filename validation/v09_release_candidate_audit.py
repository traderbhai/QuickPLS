"""QuickPLS v0.9 release-candidate gate audit."""

import glob
import json
import os
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v09_release_candidate_audit.json"
VERSION = "0.9.0-rc.1"


def run(command, timeout=240):
    started = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "elapsed_seconds": round(time.perf_counter() - started, 4),
        "stdout_tail": proc.stdout[-4000:],
        "stderr_tail": proc.stderr[-4000:],
    }


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def mtime(path):
    return path.stat().st_mtime if path.exists() else 0


def file_status(relative):
    path = ROOT / relative
    return {
        "path": relative,
        "present": path.exists(),
        "bytes": path.stat().st_size if path.exists() else 0,
    }


def package_lock_version_ok():
    path = ROOT / "package-lock.json"
    if not path.exists():
        return False
    value = read_json(path)
    root_pkg = value.get("packages", {}).get("", {})
    return value.get("version") == VERSION and root_pkg.get("version") == VERSION


def cargo_version_ok():
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    return f'version = "{VERSION}"' in text


def tauri_version_ok():
    value = read_json(ROOT / "src-tauri" / "tauri.conf.json")
    return value.get("version") == VERSION


def nsis_artifacts():
    pattern = str(ROOT / "target" / "release" / "bundle" / "nsis" / "*.exe")
    return [Path(path) for path in glob.glob(pattern)]


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    package = read_json(ROOT / "package.json")
    publication = RESULTS / "publication_ready_audit.json"
    smoke = RESULTS / "v09_smoke_check.json"
    dist_index = ROOT / "dist" / "index.html"
    release_exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    nsis = nsis_artifacts()
    docs = [
        "docs/RELEASE_NOTES_V0_9_RC1.md",
        "docs/SUPPORTED_SCOPE_V0_9_RC1.md",
        "docs/DEPENDENCY_NOTICES.md",
        "docs/KNOWN_DIFFERENCES.md",
    ]
    exports = [
        "validation/results/v09_smoke_export.csv",
        "validation/results/v09_smoke_export.html",
        "validation/results/v09_smoke_export.xlsx",
    ]
    publication_json = read_json(publication) if publication.exists() else {}
    smoke_json = read_json(smoke) if smoke.exists() else {}
    version_checks = {
        "package_json": package.get("version") == VERSION,
        "package_lock": package_lock_version_ok(),
        "cargo_workspace": cargo_version_ok(),
        "tauri": tauri_version_ok(),
    }
    artifact_checks = {
        "publication_ready_current_and_passing": publication.exists()
        and publication_json.get("passed") is True
        and publication_json.get("blocker_count") == 0
        and mtime(publication) >= mtime(ROOT / "validation" / "development_slices.json"),
        "release_executable_exists": release_exe.exists(),
        "release_executable_newer_than_frontend": release_exe.exists() and dist_index.exists() and mtime(release_exe) >= mtime(dist_index),
        "nsis_installer_exists": bool(nsis),
        "smoke_check_passing": smoke.exists() and smoke_json.get("passed") is True,
        "stable_exports_readable": all((ROOT / item).exists() and (ROOT / item).stat().st_size > 0 for item in exports),
        "docs_present": all((ROOT / item).exists() and (ROOT / item).stat().st_size > 0 for item in docs),
    }
    source_claim_checks = {
        "release_notes_no_smartpls_equivalence_claim": "does not claim unrestricted SmartPLS equivalence" in (ROOT / docs[0]).read_text(encoding="utf-8"),
        "supported_scope_states_no_project_import": "SmartPLS project files are not imported" in (ROOT / docs[1]).read_text(encoding="utf-8"),
        "release_notes_unsigned_installer": "unsigned" in (ROOT / docs[0]).read_text(encoding="utf-8").lower(),
    }
    gate = run(["cargo", "run", "-p", "qpls-cli", "--", "gate", "v0_9_publication_release_candidate"])
    gate_clear = gate["passed"] and "gates passed/open/blocked: 4/0/0" in gate["stdout_tail"] and "promotion gate: clear" in gate["stdout_tail"]
    passed = all(version_checks.values()) and all(artifact_checks.values()) and all(source_claim_checks.values()) and gate_clear
    report = {
        "schema_version": 1,
        "target": "QuickPLS v0.9.0-rc.1 release candidate",
        "passed": passed,
        "version": VERSION,
        "version_checks": version_checks,
        "artifact_checks": artifact_checks,
        "source_claim_checks": source_claim_checks,
        "nsis_installers": [str(path.relative_to(ROOT)) for path in nsis],
        "required_docs": [file_status(item) for item in docs],
        "required_exports": [file_status(item) for item in exports],
        "gate": gate,
        "note": "The RC installer is intentionally unsigned; Windows SmartScreen warnings are documented in the release notes.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
