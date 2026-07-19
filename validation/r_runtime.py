"""Helpers for development-only R reference validation.

QuickPLS does not depend on R at runtime. These helpers only support local
validation scripts that compare against R reference engines.
"""

import os
import subprocess
from pathlib import Path


def _candidate_paths():
    configured = os.environ.get("QPLS_RSCRIPT")
    if configured:
        yield configured

    roots = [
        Path.home() / "Documents" / "PLS-Sem" / "dist-desktop" / "r-runtime",
        Path.home() / "AppData" / "Local" / "Programs" / "R",
        Path("C:/Program Files/R"),
        Path("C:/Program Files (x86)/R"),
    ]
    for root in roots:
        if not root.exists():
            continue
        yield str(root / "bin" / "Rscript.exe")
        yield str(root / "bin" / "x64" / "Rscript.exe")
        for version_dir in sorted(root.glob("R-*"), reverse=True):
            yield str(version_dir / "bin" / "Rscript.exe")
            yield str(version_dir / "bin" / "x64" / "Rscript.exe")

    yield from _registry_candidates()
    yield "Rscript.exe"


def _registry_candidates():
    keys = [
        r"HKLM\SOFTWARE\R-core\R",
        r"HKLM\SOFTWARE\WOW6432Node\R-core\R",
        r"HKCU\SOFTWARE\R-core\R",
    ]
    for key in keys:
        try:
            completed = subprocess.run(
                ["reg", "query", key, "/v", "InstallPath"],
                check=True,
                capture_output=True,
                text=True,
            )
        except (OSError, subprocess.CalledProcessError):
            continue
        for line in completed.stdout.splitlines():
            if "InstallPath" not in line:
                continue
            install_path = line.split("REG_SZ", 1)[-1].strip()
            if install_path:
                yield str(Path(install_path) / "bin" / "Rscript.exe")
                yield str(Path(install_path) / "bin" / "x64" / "Rscript.exe")


def find_rscript():
    found = find_rscript_optional()
    if found is not None:
        return found
    raise SystemExit(
        "Rscript.exe was not found. Set QPLS_RSCRIPT to the full Rscript.exe path."
    )


def find_rscript_optional():
    seen = set()
    for candidate in _candidate_paths():
        if not candidate or candidate in seen:
            continue
        seen.add(candidate)
        try:
            completed = subprocess.run(
                [candidate, "--version"],
                check=True,
                capture_output=True,
                text=True,
            )
        except (OSError, subprocess.CalledProcessError):
            continue
        version = (completed.stdout or completed.stderr).strip().splitlines()[0]
        return candidate, version
    return None
