"""Probe external reference-engine availability for QuickPLS validation.

This script does not execute GPL reference engines unless their runtime is
available. It records whether the current workstation can run the development
reference scripts required by the v0.4 assessment gate.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from r_runtime import find_rscript_optional

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "external_reference_probe.json"


def command_version(command: str, env_var: str) -> dict:
    configured = os.environ.get(env_var)
    if command == "Rscript":
        found = find_rscript_optional()
        path = found[0] if found is not None else configured if configured else shutil.which(command)
    else:
        path = configured if configured else shutil.which(command)
    result = {
        "command": command,
        "env_var": env_var,
        "configured_path": configured,
        "path": path,
        "available": path is not None and Path(path).exists(),
    }
    if path is None:
        result["version"] = None
        result["error"] = f"{command} was not found on PATH and {env_var} is not set"
        return result
    if not Path(path).exists():
        result["version"] = None
        result["error"] = f"{path} does not exist"
        return result
    try:
        completed = subprocess.run(
            [path, "--version"],
            check=False,
            capture_output=True,
            text=True,
            timeout=15,
        )
        result["exit_code"] = completed.returncode
        result["version"] = (completed.stdout or completed.stderr).strip().splitlines()[:3]
    except Exception as exc:  # pragma: no cover - environment-specific failure
        result["version"] = None
        result["error"] = str(exc)
    return result


def file_status(relative: str) -> dict:
    path = ROOT / relative
    return {
        "path": relative,
        "present": path.exists(),
        "bytes": path.stat().st_size if path.exists() else None,
    }


def main() -> None:
    runtimes = [command_version("Rscript", "QPLS_RSCRIPT"), command_version("R", "QPLS_R")]
    files = [
        file_status("validation/rho_a_csem_reference.R"),
        file_status("validation/r_csem_reference.R"),
        file_status("validation/fixtures/rho_a_reference.csv"),
        file_status("validation/fixtures/corporate_reputation.csv"),
        file_status("validation/r-library/cSEM"),
    ]
    rscript_available = any(runtime["command"] == "Rscript" and runtime["available"] for runtime in runtimes)
    required_files_present = all(item["present"] for item in files)
    blocked_reasons = []
    if not rscript_available:
        blocked_reasons.append("Rscript is not available on PATH")
    if not required_files_present:
        blocked_reasons.append("one or more required reference scripts/fixtures are missing")
    report = {
        "schema_version": 1,
        "target": "v04-assessment-external-references",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "can_execute_csem_reference": rscript_available and required_files_present,
        "blocked_reasons": blocked_reasons,
        "runtimes": runtimes,
        "files": files,
        "planned_commands": [
            {
                "id": "rho_a_csem_0_6_1",
                "command": (
                    'Rscript --vanilla validation/rho_a_csem_reference.R '
                    'validation/fixtures/rho_a_reference.csv '
                    'validation/results/rho_a_csem_0_6_1.csv'
                ),
                "required_for": ["rho_a"],
                "status": "ready_to_run" if rscript_available and required_files_present else "blocked",
            },
            {
                "id": "pls_csem_variant_reference",
                "command": "Rscript --vanilla validation/r_csem_reference.R",
                "required_for": ["pls_mode_a", "pls_mode_b", "factor_weighting", "pca_weighting"],
                "status": "ready_to_run" if rscript_available and required_files_present else "blocked",
            },
        ],
        "note": (
            "This is a reproducible environment probe, not numerical validation. "
            "R/cSEM are development-only validation tools and are not runtime "
            "requirements for QuickPLS users. The v0.4 assessment external-reference "
            "gate remains open for HTMT external references and primary-paper rho_A evidence."
        ),
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {OUTPUT} | can_execute_csem_reference={report['can_execute_csem_reference']}")


if __name__ == "__main__":
    main()
