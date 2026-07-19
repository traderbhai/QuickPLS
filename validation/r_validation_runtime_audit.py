"""Audit the validation-only R runtime for publication promotion work."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "r_validation_runtime_audit.json"
RSCRIPT = Path(r"C:\Users\mohd.naved\AppData\Local\Programs\R\R-4.6.1\bin\x64\Rscript.exe")
R_LIBRARY = ROOT / "validation" / "r-library"

REQUIRED_PACKAGES = ["lavaan", "cSEM", "seminr", "plspm", "boot", "sandwich", "lmtest"]
OPTIONAL_PACKAGES = ["NCA", "semopy"]


def run_r(expr: str):
    R_LIBRARY.mkdir(parents=True, exist_ok=True)
    bootstrap = f".libPaths(c(normalizePath('{R_LIBRARY.as_posix()}', winslash='/', mustWork=FALSE), .libPaths())); "
    return subprocess.run(
        [str(RSCRIPT), "-e", bootstrap + expr],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=60,
    )


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    exists = RSCRIPT.exists()
    version = None
    packages = {}
    if exists:
        version_proc = subprocess.run(
            [str(RSCRIPT), "--version"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )
        version = (version_proc.stdout or version_proc.stderr).strip()
        names = REQUIRED_PACKAGES + OPTIONAL_PACKAGES
        escaped = ",".join(f'"{name}"' for name in names)
        expr = (
            f"pkgs <- c({escaped}); "
            "installed <- installed.packages()[, c('Package', 'Version')]; "
            "out <- lapply(pkgs, function(p) { "
            "if (p %in% installed[, 'Package']) paste(p, installed[p, 'Version'], sep='=') else paste(p, 'MISSING', sep='=')"
            "}); cat(paste(unlist(out), collapse='\\n'))"
        )
        proc = run_r(expr)
        if proc.returncode == 0:
            for line in proc.stdout.splitlines():
                if "=" in line:
                    name, value = line.split("=", 1)
                    packages[name] = None if value == "MISSING" else value
        else:
            packages["_error"] = (proc.stderr or proc.stdout).strip()
    missing_required = [name for name in REQUIRED_PACKAGES if packages.get(name) is None]
    report = {
        "schema_version": 1,
        "target": "r_validation_runtime",
        "rscript": str(RSCRIPT),
        "r_library": str(R_LIBRARY),
        "rscript_exists": exists,
        "rscript_version": version,
        "required_packages": REQUIRED_PACKAGES,
        "optional_packages": OPTIONAL_PACKAGES,
        "packages": packages,
        "missing_required_packages": missing_required,
        "passed": exists and not missing_required,
        "note": "R is validation-only and must never become a QuickPLS runtime dependency.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
