"""Guard unsupported PLSc settings.

The experimental PLSc implementation is limited to reflective constructs with
path or factor weighting. This script proves unsupported settings are rejected
before execution instead of silently falling back to ordinary PLS-PM.
"""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
FIXTURE_RECIPE = ROOT / "validation" / "fixtures" / "simple_reflective.recipe.json"
FIXTURE_DATA = ROOT / "validation" / "fixtures" / "simple_reflective.csv"
RECIPE = RESULTS / "plsc_unsupported_guard.recipe.json"
OUTPUT = RESULTS / "plsc_unsupported_guard_report.json"
RESULT = RESULTS / "plsc_unsupported_guard_quickpls.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
EXPECTED_CODE = "plsc.reflective_only"
EXPECTED_METHOD = "plsc"


def ensure_cli():
    if not CLI_EXE.exists():
        subprocess.run(
            ["cargo", "build", "-p", "qpls-cli"],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def write_recipe():
    RESULTS.mkdir(parents=True, exist_ok=True)
    recipe = json.loads(FIXTURE_RECIPE.read_text(encoding="utf-8"))
    recipe["settings"]["method"] = EXPECTED_METHOD
    recipe["model"]["constructs"][0]["mode"] = "formative"
    recipe["metadata"] = {
        **recipe.get("metadata", {}),
        "fixture": "plsc_unsupported_guard",
    }
    RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


def main():
    RESULT.unlink(missing_ok=True)
    write_recipe()
    validation = qpls(
        ["validate", str(RECIPE.relative_to(ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    issues = json.loads(validation.stdout)
    validation_codes = [issue["code"] for issue in issues]
    run = qpls(
        [
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(FIXTURE_DATA.relative_to(ROOT)),
            "--output",
            str(RESULT.relative_to(ROOT)),
            "--allow-experimental",
        ],
        capture_output=True,
        text=True,
    )
    checks = {
        "validation_exit_nonzero": validation.returncode != 0,
        "validation_code_present": EXPECTED_CODE in validation_codes,
        "run_exit_nonzero": run.returncode != 0,
        "run_error_mentions_code": EXPECTED_CODE in run.stderr or EXPECTED_CODE in run.stdout,
        "result_not_written": not RESULT.exists(),
    }
    passed = all(checks.values())
    report = {
        "schema_version": 1,
        "kind": "plsc_unsupported_guard_v1",
        "passed": passed,
        "expected_code": EXPECTED_CODE,
        "expected_method": EXPECTED_METHOD,
        "checks": checks,
        "validation_codes": validation_codes,
        "run_stderr": run.stderr,
        "note": "Experimental PLSc must reject unsupported formative/PCA settings before estimation.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed} | code={EXPECTED_CODE}")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
