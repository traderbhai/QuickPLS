"""Guard unsupported extended PLS method identifiers.

This list is intentionally empty once every planned v0.5 extended PLS method
has an experimental contract. Keep the harness so a future planned method can
be added here and proven blocked until its estimator is implemented.
"""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
FIXTURE_RECIPE = ROOT / "validation" / "fixtures" / "simple_reflective.recipe.json"
FIXTURE_DATA = ROOT / "validation" / "fixtures" / "simple_reflective.csv"
OUTPUT = RESULTS / "extended_pls_unsupported_guard_report.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
EXPECTED_CODE = "method.unsupported"
CLI_READY = False
METHODS = []


def ensure_cli():
    global CLI_READY
    if CLI_READY:
        return CLI_EXE
    subprocess.run(
        ["cargo", "build", "-p", "qpls-cli"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def write_recipe(method):
    recipe_path = RESULTS / f"extended_pls_unsupported_{method}.recipe.json"
    result_path = RESULTS / f"extended_pls_unsupported_{method}_quickpls.json"
    result_path.unlink(missing_ok=True)
    recipe = json.loads(FIXTURE_RECIPE.read_text(encoding="utf-8"))
    recipe["settings"]["method"] = method
    recipe["metadata"] = {
        **recipe.get("metadata", {}),
        "fixture": f"extended_pls_unsupported_{method}",
    }
    recipe_path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    return recipe_path, result_path


def check_method(method):
    recipe_path, result_path = write_recipe(method)
    validation = qpls(
        ["validate", str(recipe_path.relative_to(ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    issues = json.loads(validation.stdout)
    validation_codes = [issue["code"] for issue in issues]
    validation_subjects = [issue.get("subject") for issue in issues]
    run = qpls(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(FIXTURE_DATA.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        capture_output=True,
        text=True,
    )
    checks = {
        "validation_exit_nonzero": validation.returncode != 0,
        "validation_code_present": EXPECTED_CODE in validation_codes,
        "validation_subject_present": method in validation_subjects,
        "run_exit_nonzero": run.returncode != 0,
        "run_error_mentions_code": EXPECTED_CODE in run.stderr or EXPECTED_CODE in run.stdout,
        "result_not_written": not result_path.exists(),
    }
    return {
        "method": method,
        "passed": all(checks.values()),
        "checks": checks,
        "validation_codes": validation_codes,
        "validation_subjects": validation_subjects,
        "run_stderr": run.stderr,
    }


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    rows = [check_method(method) for method in METHODS]
    passed = all(row["passed"] for row in rows)
    report = {
        "schema_version": 1,
        "kind": "extended_pls_unsupported_guard_v1",
        "passed": passed,
        "expected_code": EXPECTED_CODE,
        "methods": METHODS,
        "results": rows,
        "note": "No v0.5 extended PLS method ids are currently pending as unsupported; add future planned ids here until contracts and references are implemented.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed} | methods={len(METHODS)}")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
