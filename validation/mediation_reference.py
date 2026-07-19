"""Independent mediation reference fixture for QuickPLS.

The fixture uses single-item constructs so the PLS scores reduce to documented
sample-standardized observed variables. Structural paths and indirect effects
are then computed with plain OLS equations outside the QuickPLS engine.
"""

import csv
import json
import math
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "mediation_reference.csv"
RECIPE = RESULTS / "mediation_reference.recipe.json"
QUICKPLS = RESULTS / "mediation_reference_quickpls.json"
OUTPUT = RESULTS / "mediation_reference_report.json"


def standardize(values):
    mean = sum(values) / len(values)
    centered = [value - mean for value in values]
    variance = sum(value * value for value in centered) / (len(values) - 1)
    scale = math.sqrt(variance)
    if not math.isfinite(scale) or scale <= sys_float_epsilon():
        raise ValueError("cannot standardize a zero-variance vector")
    return [value / scale for value in centered]


def sys_float_epsilon():
    return 2.220446049250313e-16


def covariance(left, right):
    return sum(a * b for a, b in zip(left, right)) / (len(left) - 1)


def regression_slope(predictor, outcome):
    return covariance(predictor, outcome) / covariance(predictor, predictor)


def write_dataset():
    rows = [
        {"x": 1.0, "m": 2.0, "y": 3.0},
        {"x": 2.0, "m": 3.0, "y": 5.0},
        {"x": 3.0, "m": 5.0, "y": 8.0},
        {"x": 4.0, "m": 7.0, "y": 11.0},
        {"x": 5.0, "m": 11.0, "y": 16.0},
        {"x": 6.0, "m": 13.0, "y": 19.0},
        {"x": 7.0, "m": 17.0, "y": 24.0},
        {"x": 8.0, "m": 19.0, "y": 27.0},
    ]
    RESULTS.mkdir(parents=True, exist_ok=True)
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x", "m", "y"])
        writer.writeheader()
        writer.writerows(rows)
    return rows


def dataset_fingerprint():
    project_path = RESULTS / "mediation_reference.fingerprint.qpls"
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "import",
            str(DATA.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            "mediation_reference",
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "inspect",
            str(project_path.relative_to(ROOT)),
            "--json",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    project_path.unlink(missing_ok=True)
    return json.loads(completed.stdout)["datasets"][0]["fingerprint"]


def write_recipe(fingerprint):
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000055",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000056",
            "name": "Independent mediation reference",
            "constructs": [
                {
                    "id": "x",
                    "name": "X",
                    "short_name": "X",
                    "mode": "reflective",
                    "indicators": ["x"],
                },
                {
                    "id": "m",
                    "name": "M",
                    "short_name": "M",
                    "mode": "reflective",
                    "indicators": ["m"],
                },
                {
                    "id": "y",
                    "name": "Y",
                    "short_name": "Y",
                    "mode": "reflective",
                    "indicators": ["y"],
                },
            ],
            "paths": [
                {"source": "x", "target": "m"},
                {"source": "m", "target": "y"},
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
        "metadata": {"fixture": "independent mediation effect equations"},
    }
    RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


def run_quickpls():
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
            "--allow-experimental",
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def independent_reference(rows):
    x = standardize([row["x"] for row in rows])
    m = standardize([row["m"] for row in rows])
    y = standardize([row["y"] for row in rows])
    path_xm = regression_slope(x, m)
    path_my = regression_slope(m, y)
    indirect_xy = path_xm * path_my
    return {
        ("x", "m"): {"direct": path_xm, "indirect": 0.0, "total": path_xm, "vaf": 0.0},
        ("m", "y"): {"direct": path_my, "indirect": 0.0, "total": path_my, "vaf": 0.0},
        ("x", "y"): {"direct": 0.0, "indirect": indirect_xy, "total": indirect_xy, "vaf": 1.0},
    }


def compare(reference, quickpls):
    estimates = quickpls["payload"]["estimation"]["mediation"]["estimates"]
    by_pair = {(row["source"], row["target"]): row for row in estimates}
    differences = []
    for pair, expected in reference.items():
        actual = by_pair[pair]
        for metric in ["direct", "indirect", "total"]:
            differences.append(
                {
                    "pair": list(pair),
                    "metric": metric,
                    "expected": expected[metric],
                    "actual": actual[metric],
                    "abs_difference": abs(expected[metric] - actual[metric]),
                }
            )
        differences.append(
            {
                "pair": list(pair),
                "metric": "variance_accounted_for",
                "expected": expected["vaf"],
                "actual": actual["variance_accounted_for"],
                "abs_difference": abs(expected["vaf"] - actual["variance_accounted_for"]),
            }
        )
    max_abs_difference = max(row["abs_difference"] for row in differences)
    return differences, max_abs_difference


def main():
    rows = write_dataset()
    write_recipe(dataset_fingerprint())
    quickpls = run_quickpls()
    reference = independent_reference(rows)
    differences, max_abs_difference = compare(reference, quickpls)
    report = {
        "schema_version": 1,
        "kind": "pls_mediation_reference_v1",
        "method_version": quickpls["payload"]["estimation"]["mediation"]["method_version"],
        "source_data": str(DATA.relative_to(ROOT)).replace("\\", "/"),
        "source_recipe": str(RECIPE.relative_to(ROOT)).replace("\\", "/"),
        "quickpls_result": str(QUICKPLS.relative_to(ROOT)).replace("\\", "/"),
        "tolerance": 1e-12,
        "passed": max_abs_difference <= 1e-12,
        "max_abs_difference": max_abs_difference,
        "differences": differences,
        "note": "Independent single-item mediation fixture; not a full publication-validation suite.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']} | max_abs_difference={max_abs_difference:.3g}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
