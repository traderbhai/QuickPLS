"""Independent bounded CCA composite-correlation residual reference.

The fixture is deliberately non-saturated. It validates descriptive residual
reconstruction only; it does not manufacture a CCA threshold, decision,
classification, or inference contract.
"""

import copy
import csv
import json
import math
import random
import subprocess
from pathlib import Path

import numpy as np

from higher_order_reference import estimate_pls


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "cca_reference.csv"
RECIPE = RESULTS / "cca_reference.recipe.json"
QUICKPLS = RESULTS / "cca_reference_quickpls.json"
OUTPUT = RESULTS / "cca_reference_report.json"
GUARD_RECIPE = RESULTS / "cca_invalid.recipe.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
TOLERANCE = 1e-10
MINIMUM_NONSATURATED_RESIDUAL = 0.05
CCA_METHOD_VERSION = "cca_composite_residual_v1"
CCA_MODEL_VERSION = "recursive_standardized_composite_path_model_v1"
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


def generated_rows(seed=20260719, n=132):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        z = 0.45 * x + rng.gauss(0.0, 0.75)
        y = 0.35 * x + 0.55 * z + rng.gauss(0.0, 0.50)
        rows.append(
            {
                "x1": 0.91 * x + rng.gauss(0.0, 0.16),
                "x2": 0.84 * x + rng.gauss(0.0, 0.19),
                "z1": 0.88 * z + rng.gauss(0.0, 0.17),
                "z2": 0.80 * z + rng.gauss(0.0, 0.21),
                "y1": 0.93 * y + rng.gauss(0.0, 0.15),
                "y2": 0.79 * y + rng.gauss(0.0, 0.22),
            }
        )
    return rows


def write_dataset(rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    fields = ["x1", "x2", "z1", "z2", "y1", "y2"]
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: f"{row[field]:.12f}" for field in fields})


def dataset_fingerprint():
    project_path = RESULTS / "cca_reference.fingerprint.qpls"
    qpls(
        ["import", str(DATA.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", "cca_reference"],
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
        "id": "00000000-0000-0000-0000-000000000017",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000018",
            "name": "CCA composite residual reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [
                {"source": "x", "target": "z"},
                {"source": "z", "target": "y"},
            ],
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        },
        "settings": {
            "method": "cca",
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
            "case_weight_column": None,
        },
        "metadata": {"fixture": "independent_nonsaturated_cca_reference"},
    }


def correlation(left, right):
    return float(np.cov(np.asarray(left), np.asarray(right), ddof=1)[0, 1] / (np.std(left, ddof=1) * np.std(right, ddof=1)))


def reference_cca(rows, recipe):
    columns = {
        name: [row[name] for row in rows]
        for name in ["x1", "x2", "z1", "z2", "y1", "y2"]
    }
    pls = estimate_pls(columns, recipe)
    construct_ids = [construct["id"] for construct in recipe["model"]["constructs"]]
    index = {construct: position for position, construct in enumerate(construct_ids)}
    count = len(construct_ids)
    observed = np.eye(count)
    for row in range(count):
        for column in range(row):
            value = correlation(pls["scores"][construct_ids[row]], pls["scores"][construct_ids[column]])
            observed[row, column] = value
            observed[column, row] = value
    structural = np.zeros((count, count))
    for (source, target), coefficient in pls["paths"].items():
        structural[index[target], index[source]] = coefficient
    system = np.eye(count) - structural
    endogenous = {path["target"] for path in recipe["model"]["paths"]}
    residual_covariance = np.zeros((count, count))
    for row, construct_id in enumerate(construct_ids):
        if construct_id in endogenous:
            predecessors = [path["source"] for path in recipe["model"]["paths"] if path["target"] == construct_id]
            r2 = sum(pls["paths"][(source, construct_id)] * observed[index[source], row] for source in predecessors)
            residual_covariance[row, row] = max(0.0, 1.0 - r2)
        else:
            residual_covariance[row, row] = 1.0
            for column in range(row):
                if construct_ids[column] not in endogenous:
                    residual_covariance[row, column] = observed[row, column]
                    residual_covariance[column, row] = observed[row, column]
    inverse = np.linalg.solve(system, np.eye(count))
    reproduced = inverse @ residual_covariance @ inverse.T
    rows_out = {}
    for row in range(count):
        for column in range(row):
            key = (construct_ids[column], construct_ids[row])
            rows_out[key] = {
                "observed": float(observed[row, column]),
                "reproduced": float(reproduced[row, column]),
                "residual": float(observed[row, column] - reproduced[row, column]),
                "absolute_residual": abs(float(observed[row, column] - reproduced[row, column])),
            }
    return rows_out


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
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def validate_recipe(recipe):
    GUARD_RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    validation = qpls(
        ["validate", str(GUARD_RECIPE.relative_to(ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    issues = json.loads(validation.stdout)
    return validation.returncode, [issue["code"] for issue in issues]


def check_guard(fingerprint, name, expected_code, mutate):
    recipe = copy.deepcopy(recipe_payload(fingerprint))
    recipe["metadata"]["fixture"] = f"invalid_cca_{name}"
    mutate(recipe)
    returncode, codes = validate_recipe(recipe)
    return {
        "passed": returncode != 0 and expected_code in codes,
        "expected_code": expected_code,
        "validation_codes": codes,
    }


def check_guards(fingerprint):
    guards = {
        "pca_weighting": check_guard(
            fingerprint,
            "pca_weighting",
            "cca.pca_unsupported",
            lambda recipe: recipe["settings"].update(weighting_scheme="pca"),
        ),
        "nonstandardized_preprocessing": check_guard(
            fingerprint,
            "nonstandardized_preprocessing",
            "cca.standardized_required",
            lambda recipe: recipe["settings"].update(preprocessing="mean_centered"),
        ),
        "too_few_constructs": check_guard(
            fingerprint,
            "too_few_constructs",
            "cca.constructs_required",
            lambda recipe: (
                recipe["model"].update(constructs=recipe["model"]["constructs"][:1], paths=[])
            ),
        ),
        "no_structural_path": check_guard(
            fingerprint,
            "no_structural_path",
            "cca.structural_path_required",
            lambda recipe: recipe["model"].update(paths=[]),
        ),
        "formative_construct": check_guard(
            fingerprint,
            "formative_construct",
            "cca.reflective_only",
            lambda recipe: recipe["model"]["constructs"][0].update(mode="formative"),
        ),
        "empty_indicator_block": check_guard(
            fingerprint,
            "empty_indicator_block",
            "cca.observed_indicators_required",
            lambda recipe: recipe["model"]["constructs"][0].update(indicators=[]),
        ),
        "interaction": check_guard(
            fingerprint,
            "interaction",
            "cca.interactions_unsupported",
            lambda recipe: recipe["model"].update(interactions=[{
                "id": "x_by_z",
                "predictor": "x",
                "moderator": "z",
                "product_construct": "x_by_z",
                "outcome": "y",
                "method": "two_stage_product_score",
            }]),
        ),
        "higher_order_construct": check_guard(
            fingerprint,
            "higher_order_construct",
            "cca.higher_order_unsupported",
            lambda recipe: recipe["model"].update(higher_order_constructs=[{
                "id": "y",
                "components": ["x", "z"],
                "method": "repeated_indicators",
                "stage_one_recipe": None,
            }]),
        ),
        "control_path": check_guard(
            fingerprint,
            "control_path",
            "cca.controls_unsupported",
            lambda recipe: recipe["model"].update(controls=[{
                "source": "x",
                "target": "z",
                "label": "Control",
            }]),
        ),
        "case_weights": check_guard(
            fingerprint,
            "case_weights",
            "cca.case_weights_unsupported",
            lambda recipe: recipe["settings"].update(case_weight_column="x1"),
        ),
        "resampling": check_guard(
            fingerprint,
            "resampling",
            "cca.resampling_unsupported",
            lambda recipe: recipe["settings"].update(
                bootstrap_samples=999,
                studentized_inner_samples=99,
                permutation_samples=99,
            ),
        ),
    }

    factor_recipe = copy.deepcopy(recipe_payload(fingerprint))
    factor_recipe["settings"]["weighting_scheme"] = "factor"
    factor_recipe["metadata"]["fixture"] = "valid_cca_factor_weighting"
    factor_returncode, factor_codes = validate_recipe(factor_recipe)
    guards["factor_weighting_allowed"] = {
        "passed": factor_returncode == 0,
        "validation_codes": factor_codes,
    }
    guards["listwise_policy_fixed"] = {
        "passed": recipe_payload(fingerprint)["settings"]["missing_data"] == "listwise_deletion",
        "detail": "Listwise deletion is the only currently representable missing-data policy; the Rust contract retains a future-proof cca.listwise_required guard.",
    }
    return guards


def main():
    rows = generated_rows()
    write_dataset(rows)
    fingerprint = dataset_fingerprint()
    recipe = recipe_payload(fingerprint)
    RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    quickpls = run_quickpls()
    estimation = quickpls["payload"]["estimation"]
    analysis = estimation["cca"]
    provenance = quickpls["provenance"]
    expected = reference_cca(rows, recipe)
    observed = {
        (row["left"], row["right"]): row
        for row in analysis["correlations"]
    }
    deltas = {}
    for key, values in expected.items():
        for field, value in values.items():
            deltas[f"{field}::{key[0]}::{key[1]}"] = abs(observed[key][field] - value)
    deltas["max_absolute_residual"] = abs(
        analysis["max_absolute_residual"] - max(row["absolute_residual"] for row in expected.values())
    )
    max_delta = max(deltas.values())
    guards = check_guards(fingerprint)
    expected_pairs = set(expected)
    observed_pairs = set(observed)
    correlations_finite = all(
        math.isfinite(row[field])
        for row in analysis["correlations"]
        for field in ["observed", "reproduced", "residual", "absolute_residual"]
    )
    row_algebra_valid = all(
        abs(row["residual"] - (row["observed"] - row["reproduced"])) <= TOLERANCE
        and abs(row["absolute_residual"] - abs(row["residual"])) <= TOLERANCE
        for row in analysis["correlations"]
    )
    expected_payload_keys = {
        "method_version",
        "model",
        "correlations",
        "max_absolute_residual",
        "warnings",
    }
    expected_row_keys = {
        "left",
        "right",
        "observed",
        "reproduced",
        "residual",
        "absolute_residual",
    }
    provenance_versions = provenance["method_version"].split("+")
    checks = {
        "method": provenance["method"] == "cca" and provenance["settings"]["method"] == "cca",
        "method_version": estimation["method_version"] == CCA_METHOD_VERSION,
        "payload_version": analysis["method_version"] == CCA_METHOD_VERSION,
        "provenance_includes_payload_version": CCA_METHOD_VERSION in provenance_versions,
        "nested_model_version": analysis["model"] == CCA_MODEL_VERSION,
        "nonsaturated_fixture": len(recipe["model"]["paths"]) == 2
        and len(analysis["correlations"]) == 3
        and analysis["max_absolute_residual"] >= MINIMUM_NONSATURATED_RESIDUAL,
        "exact_pair_identity": observed_pairs == expected_pairs,
        "correlation_count": len(analysis["correlations"]) == 3,
        "finite_rows": correlations_finite and math.isfinite(analysis["max_absolute_residual"]),
        "residual_algebra": row_algebra_valid,
        "max_residual_identity": abs(
            analysis["max_absolute_residual"]
            - max(row["absolute_residual"] for row in analysis["correlations"])
        ) <= TOLERANCE,
        "descriptive_payload_only": set(analysis) == expected_payload_keys
        and all(set(row) == expected_row_keys for row in analysis["correlations"]),
        "max_delta_within_tolerance": max_delta <= TOLERANCE,
        "bounded_scope_guards": all(guard["passed"] for guard in guards.values()),
    }
    report = {
        "schema_version": 1,
        "kind": "cca_reference_v1",
        "passed": all(checks.values()),
        "tolerance": TOLERANCE,
        "minimum_nonsaturated_residual": MINIMUM_NONSATURATED_RESIDUAL,
        "max_delta": max_delta,
        "max_absolute_residual": analysis["max_absolute_residual"],
        "method_version": CCA_METHOD_VERSION,
        "nested_model_version": CCA_MODEL_VERSION,
        "fixture": {
            "constructs": [construct["id"] for construct in recipe["model"]["constructs"]],
            "paths": recipe["model"]["paths"],
            "correlation_pairs": sorted([list(pair) for pair in expected_pairs]),
            "saturated": False,
        },
        "checks": checks,
        "deltas": deltas,
        "guards": guards,
        "scope": {
            "kind": "descriptive_composite_correlation_residuals_only",
            "inference": False,
            "thresholds": False,
            "classification": False,
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']} | max_delta={max_delta:.3g}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
