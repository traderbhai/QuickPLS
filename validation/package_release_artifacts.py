"""Copy release desktop outputs to unique versioned artifact names.

Tauri writes predictable bundle names under target/release and may overwrite
the previous installer for the same app version. This script preserves a fresh
copy for user testing by adding the app version, milestone label, and UTC build
timestamp to each artifact name.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PACKAGE_JSON = ROOT / "package.json"
TAURI_CONFIG = ROOT / "src-tauri" / "tauri.conf.json"
RELEASE_DIR = ROOT / "target" / "release"
NSIS_DIR = RELEASE_DIR / "bundle" / "nsis"
ARTIFACT_DIR = RELEASE_DIR / "artifacts"
RESULTS = ROOT / "validation" / "results"
REPORT = RESULTS / "release_artifacts.json"


def slug(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9_.-]+", "_", value.strip())
    return cleaned.strip("._-") or "build"


def read_version() -> str:
    package = json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI_CONFIG.read_text(encoding="utf-8"))
    package_version = package["version"]
    tauri_version = tauri["version"]
    if package_version != tauri_version:
        raise SystemExit(f"Version mismatch: package.json={package_version}, tauri.conf.json={tauri_version}")
    return package_version


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def copy_artifact(source: Path, destination: Path) -> dict[str, object]:
    if not source.exists():
        raise SystemExit(f"Missing release artifact: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    return {
        "source": str(source.relative_to(ROOT)),
        "path": str(destination.relative_to(ROOT)),
        "bytes": destination.stat().st_size,
        "sha256": sha256(destination),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", default="manual_release", help="Milestone/build label to include in artifact names.")
    parser.add_argument("--timestamp", default=None, help="Optional UTC timestamp override, e.g. 20260722-120000.")
    args = parser.parse_args()

    version = read_version()
    label = slug(args.label)
    timestamp = slug(args.timestamp or datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S"))
    stem = f"QuickPLS_{version}_{label}_{timestamp}_x64"

    release_exe = RELEASE_DIR / "quickpls-desktop.exe"
    installer_candidates = sorted(NSIS_DIR.glob(f"QuickPLS_{version}_*_setup.exe")) + sorted(NSIS_DIR.glob(f"QuickPLS_{version}_*setup.exe"))
    if not installer_candidates:
        installer_candidates = sorted(NSIS_DIR.glob("QuickPLS_*_x64-setup.exe"))
    if not installer_candidates:
        raise SystemExit(f"No NSIS installer found in {NSIS_DIR}")
    installer = max(installer_candidates, key=lambda path: path.stat().st_mtime)

    artifacts = [
        copy_artifact(release_exe, ARTIFACT_DIR / f"{stem}_portable.exe"),
        copy_artifact(installer, ARTIFACT_DIR / f"{stem}_setup.exe"),
    ]
    checksum_path = ARTIFACT_DIR / f"{stem}_checksums.txt"
    checksum_text = "\n".join(f"{item['sha256']}  {Path(str(item['path'])).name}" for item in artifacts) + "\n"
    checksum_path.write_text(checksum_text, encoding="utf-8")
    artifacts.append({
        "source": None,
        "path": str(checksum_path.relative_to(ROOT)),
        "bytes": checksum_path.stat().st_size,
        "sha256": sha256(checksum_path),
    })

    report = {
        "schema_version": 1,
        "target": "QuickPLS release artifact preservation",
        "passed": True,
        "version": version,
        "label": label,
        "timestamp_utc": timestamp,
        "artifact_directory": str(ARTIFACT_DIR.relative_to(ROOT)),
        "artifacts": artifacts,
        "note": "Files are copied to unique names so repeated desktop builds do not overwrite prior user-testable artifacts.",
    }
    RESULTS.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
