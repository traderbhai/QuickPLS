"""Guard invalid hybrid higher-order construct execution.

Hybrid HOC estimation is implemented experimentally through an indicator-split
contract. This validation fixture proves that a recipe with a component that
cannot be split is rejected before estimation.
"""

import csv
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "higher_order_hybrid_guard_report.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
EXPECTED_CODE = "higher_order.hybrid_component_indicators"
CLI_READY = False


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


def write_csv(path):
    RESULTS.mkdir(parents=True, exist_ok=True)
    rows = [
        {"x1": 1.0, "x2": 1.2, "z1": 2.0, "z2": 2.1, "y1": 3.2},
        {"x1": 2.0, "x2": 2.2, "z1": 2.4, "z2": 2.7, "y1": 4.4},
        {"x1": 3.0, "x2": 3.2, "z1": 3.6, "z2": 3.8, "y1": 6.0},
        {"x1": 4.0, "x2": 4.1, "z1": 4.7, "z2": 5.0, "y1": 7.8},
        {"x1": 5.0, "x2": 5.3, "z1": 5.8, "z2": 6.1, "y1": 9.5},
    ]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x1", "x2", "z1", "z2", "y1"])
        writer.writeheader()
        writer.writerows(rows)


def dataset_fingerprint(csv_path, stem):
    project_path = RESULTS / f"{stem}.fingerprint.qpls"
    qpls(
        [
            "import",
            str(csv_path.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            stem,
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    completed = qpls(
        ["inspect", str(project_path.relative_to(ROOT)), "--json"],
        check=True,
        capture_output=True,
        text=True,
    )
    project_path.unlink(missing_ok=True)
    return json.loads(completed.stdout)["datasets"][0]["fingerprint"]


def write_recipe(path, fingerprint):
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000091",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000092",
            "name": "Hybrid HOC guard",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1"]},
                {"id": "hoc", "name": "HOC", "short_name": "HOC", "mode": "reflective", "indicators": []},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1"]},
            ],
            "paths": [{"source": "hoc", "target": "y"}],
            "higher_order_constructs": [
                {
                    "id": "hoc",
                    "components": ["x", "z"],
                    "method": "hybrid",
                    "stage_one_recipe": None,
                }
            ],
        },
        "settings": {
            "method": "pls_pm",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": 20260719,
            "workers": 1,
            "confidence_level": 0.95,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "higher_order_hybrid_guard"},
    }
    path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


def main():
    stem = "higher_order_hybrid_guard"
    csv_path = RESULTS / f"{stem}.csv"
    recipe_path = RESULTS / f"{stem}.recipe.json"
    result_path = RESULTS / f"{stem}_quickpls.json"
    write_csv(csv_path)
    write_recipe(recipe_path, dataset_fingerprint(csv_path, stem))

    validation = qpls(
        ["validate", str(recipe_path.relative_to(ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    issues = json.loads(validation.stdout)
    validation_codes = [issue["code"] for issue in issues]
    run = qpls(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(csv_path.relative_to(ROOT)),
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
        "run_exit_nonzero": run.returncode != 0,
        "run_error_mentions_code": EXPECTED_CODE in run.stderr or EXPECTED_CODE in run.stdout,
        "result_not_written": not result_path.exists(),
    }
    passed = all(checks.values())
    report = {
        "schema_version": 1,
        "kind": "hybrid_hoc_invalid_split_guard_v1",
        "passed": passed,
        "expected_code": EXPECTED_CODE,
        "checks": checks,
        "validation_codes": validation_codes,
        "run_stderr": run.stderr,
        "note": "Hybrid HOC is experimental; validation and execution must block components that cannot be split into lower-order and higher-order indicator blocks.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed} | code={EXPECTED_CODE}")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
