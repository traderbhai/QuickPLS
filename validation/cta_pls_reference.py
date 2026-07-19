"""Independent CTA-PLS tetrad diagnostic reference."""

import csv
import json
import math
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "cta_pls_reference.csv"
RECIPE = RESULTS / "cta_pls_reference.recipe.json"
QUICKPLS = RESULTS / "cta_pls_reference_quickpls.json"
OUTPUT = RESULTS / "cta_pls_reference_report.json"
GUARD_RECIPE = RESULTS / "cta_pls_invalid.recipe.json"
GUARD_RESULT = RESULTS / "cta_pls_invalid_quickpls.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
TOLERANCE = 1e-10


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


def generated_rows(seed=20260719, n=120):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        y = 0.55 * x + rng.gauss(0.0, 0.55)
        rows.append(
            {
                "x1": 0.92 * x + rng.gauss(0.0, 0.18),
                "x2": 0.86 * x + rng.gauss(0.0, 0.20),
                "x3": 0.80 * x + rng.gauss(0.0, 0.22),
                "x4": 0.76 * x + rng.gauss(0.0, 0.24),
                "y1": 0.90 * y + rng.gauss(0.0, 0.16),
                "y2": 0.84 * y + rng.gauss(0.0, 0.18),
            }
        )
    return rows


def write_dataset(rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    fields = ["x1", "x2", "x3", "x4", "y1", "y2"]
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: f"{row[field]:.12f}" for field in fields})


def dataset_fingerprint():
    project_path = RESULTS / "cta_pls_reference.fingerprint.qpls"
    qpls(
        [
            "import",
            str(DATA.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            "cta_pls_reference",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    payload = json.loads(
        qpls(
            ["inspect", str(project_path.relative_to(ROOT)), "--json"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    return payload["datasets"][0]["fingerprint"]


def recipe_payload(fingerprint):
    return {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000013",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000014",
            "name": "CTA-PLS tetrad reference",
            "constructs": [
                {
                    "id": "x",
                    "name": "X",
                    "short_name": "X",
                    "mode": "reflective",
                    "indicators": ["x1", "x2", "x3", "x4"],
                },
                {
                    "id": "y",
                    "name": "Y",
                    "short_name": "Y",
                    "mode": "reflective",
                    "indicators": ["y1", "y2"],
                },
            ],
            "paths": [{"source": "x", "target": "y"}],
        },
        "settings": {
            "method": "cta_pls",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "independent_cta_pls_tetrad_reference"},
    }


def standardize(values):
    mean = sum(values) / len(values)
    centered = [value - mean for value in values]
    variance = sum(value * value for value in centered) / (len(values) - 1)
    scale = math.sqrt(variance)
    return [value / scale for value in centered]


def covariance(left, right):
    left_mean = sum(left) / len(left)
    right_mean = sum(right) / len(right)
    return sum((a - left_mean) * (b - right_mean) for a, b in zip(left, right)) / (len(left) - 1)


def reference_tetrads(rows):
    columns = {
        field: standardize([row[field] for row in rows])
        for field in ["x1", "x2", "x3", "x4", "y1", "y2"]
    }
    cov_ab = covariance(columns["x1"], columns["x2"])
    cov_ac = covariance(columns["x1"], columns["x3"])
    cov_ad = covariance(columns["x1"], columns["x4"])
    cov_bc = covariance(columns["x2"], columns["x3"])
    cov_bd = covariance(columns["x2"], columns["x4"])
    cov_cd = covariance(columns["x3"], columns["x4"])
    values = {
        ("x", "x1", "x2", "x3", "x4", "ab_cd_minus_ac_bd"): cov_ab * cov_cd - cov_ac * cov_bd,
        ("x", "x1", "x2", "x3", "x4", "ac_bd_minus_ad_bc"): cov_ac * cov_bd - cov_ad * cov_bc,
        ("x", "x1", "x2", "x3", "x4", "ad_bc_minus_ab_cd"): cov_ad * cov_bc - cov_ab * cov_cd,
    }
    return values


def run_quickpls():
    QUICKPLS.unlink(missing_ok=True)
    qpls(
        [
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
            "--allow-experimental",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def check_guard(fingerprint):
    recipe = recipe_payload(fingerprint)
    recipe["model"]["constructs"][0]["indicators"] = ["x1", "x2", "x3"]
    recipe["metadata"]["fixture"] = "invalid_cta_pls_three_indicator_block"
    GUARD_RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    GUARD_RESULT.unlink(missing_ok=True)
    validation = qpls(
        ["validate", str(GUARD_RECIPE.relative_to(ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    issues = json.loads(validation.stdout)
    run = qpls(
        [
            "run",
            str(GUARD_RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(GUARD_RESULT.relative_to(ROOT)),
            "--allow-experimental",
        ],
        capture_output=True,
        text=True,
    )
    codes = [issue["code"] for issue in issues]
    return {
        "passed": validation.returncode != 0
        and "cta_pls.tetrad_block_required" in codes
        and run.returncode != 0
        and not GUARD_RESULT.exists(),
        "validation_codes": codes,
        "run_stderr": run.stderr,
    }


def main():
    rows = generated_rows()
    write_dataset(rows)
    fingerprint = dataset_fingerprint()
    RECIPE.write_text(json.dumps(recipe_payload(fingerprint), indent=2) + "\n", encoding="utf-8")
    quickpls = run_quickpls()
    estimation = quickpls["payload"]["estimation"]
    analysis = estimation["cta_pls"]
    observed = {
        (
            item["construct"],
            item["indicator_a"],
            item["indicator_b"],
            item["indicator_c"],
            item["indicator_d"],
            item["pairing"],
        ): item["tetrad"]
        for item in analysis["estimates"]
    }
    expected = reference_tetrads(rows)
    deltas = {
        "::".join(key): abs(observed[key] - expected[key])
        for key in expected
    }
    max_delta = max(deltas.values())
    guard = check_guard(fingerprint)
    checks = {
        "method_version": estimation["method_version"] == "cta_pls_tetrad_v1",
        "payload_version": analysis["method_version"] == "cta_pls_tetrad_v1",
        "estimate_count": len(analysis["estimates"]) == 3,
        "max_delta_within_tolerance": max_delta <= TOLERANCE,
        "max_absolute_summary": abs(
            analysis["max_absolute_tetrad_by_construct"]["x"] - max(abs(value) for value in expected.values())
        )
        <= TOLERANCE,
        "guard": guard["passed"],
    }
    report = {
        "schema_version": 1,
        "kind": "cta_pls_reference_v1",
        "passed": all(checks.values()),
        "tolerance": TOLERANCE,
        "max_delta": max_delta,
        "checks": checks,
        "deltas": deltas,
        "guard": guard,
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']} | max_delta={max_delta:.3g}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
