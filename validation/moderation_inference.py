"""Moderation inference smoke validation for QuickPLS.

This script verifies that the existing PLS bootstrap, BCa, and Freedman-Lane
permutation engines carry the generated two-stage moderation product path as a
normal path parameter. It is deliberately bounded; it is not a full statistical
coverage qualification.
"""

import json
import math
from subprocess import DEVNULL

from moderation_reference import (
    ROOT,
    RESULTS,
    dataset_fingerprint,
    generated_rows,
    independent_reference,
    qpls_cli,
    simple_slopes,
    write_csv,
    write_recipe,
)


OUTPUT = RESULTS / "moderation_inference_report.json"
BOOTSTRAP_SAMPLES = 99
PERMUTATION_SAMPLES = 99
TOLERANCE = 1e-10


def run_quickpls(stem, workers):
    rows = generated_rows()
    csv_path = RESULTS / f"{stem}.csv"
    recipe_path = RESULTS / f"{stem}.recipe.json"
    result_path = RESULTS / f"{stem}_quickpls.json"
    write_csv(csv_path, rows)
    write_recipe(
        recipe_path,
        dataset_fingerprint(csv_path, stem),
        stem,
        ("x", "m", "xm", "y"),
    )
    qpls_cli(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(csv_path.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
            "--bootstrap-samples",
            str(BOOTSTRAP_SAMPLES),
            "--permutation-samples",
            str(PERMUTATION_SAMPLES),
            "--workers",
            str(workers),
        ],
        stdout=DEVNULL,
    )
    return json.loads(result_path.read_text(encoding="utf-8"))


def parameter_key(kind, parts):
    return json.dumps([kind, parts], separators=(",", ":"))


def find_parameter(rows, kind, parts):
    key = parameter_key(kind, parts)
    for row in rows:
        if row["parameter"] == key:
            return row
    raise AssertionError(f"missing parameter {key}")


def assert_close(left, right, path="$"):
    if isinstance(left, dict) and isinstance(right, dict):
        if set(left) != set(right):
            raise AssertionError(f"{path}: key mismatch {set(left) ^ set(right)}")
        for key in sorted(left):
            assert_close(left[key], right[key], f"{path}.{key}")
    elif isinstance(left, list) and isinstance(right, list):
        if len(left) != len(right):
            raise AssertionError(f"{path}: length mismatch {len(left)} != {len(right)}")
        for index, (left_item, right_item) in enumerate(zip(left, right)):
            assert_close(left_item, right_item, f"{path}[{index}]")
    elif isinstance(left, (int, float)) and isinstance(right, (int, float)):
        if not (math.isfinite(left) and math.isfinite(right)):
            if left != right:
                raise AssertionError(f"{path}: non-finite mismatch {left} != {right}")
        elif abs(left - right) > TOLERANCE:
            raise AssertionError(f"{path}: {left} != {right}")
    else:
        if left != right:
            raise AssertionError(f"{path}: {left!r} != {right!r}")


def payload_without_worker_specific_values(result):
    payload = json.loads(json.dumps(result["payload"]))
    payload["bootstrap"]["plan"]["master_seed"] = result["payload"]["bootstrap"]["plan"]["master_seed"]
    return payload


def main():
    one_worker = run_quickpls("moderation_inference_workers_1", 1)
    two_workers = run_quickpls("moderation_inference_workers_2", 2)
    assert_close(
        payload_without_worker_specific_values(one_worker),
        payload_without_worker_specific_values(two_workers),
    )

    estimation = one_worker["payload"]["estimation"]
    moderation = estimation["moderation"]["estimates"][0]
    reference = independent_reference(generated_rows())
    expected_slopes = simple_slopes(reference)
    actual_slopes = {
        row["moderator_score"]: row["effect"] for row in moderation["simple_slopes"]
    }
    slope_delta = max(
        abs(expected - actual_slopes[level])
        for level, expected in expected_slopes.items()
    )

    path_parts = ["xm", "y"]
    percentile = find_parameter(
        one_worker["payload"]["bootstrap"]["percentile"]["parameters"],
        "path",
        path_parts,
    )
    bca = find_parameter(
        one_worker["payload"]["bootstrap"]["bca"]["parameters"],
        "path",
        path_parts,
    )
    permutation = find_parameter(
        one_worker["payload"]["permutation"]["parameters"],
        "path",
        path_parts,
    )

    original_delta = abs(percentile["original"] - moderation["interaction_effect"])
    reference_delta = abs(moderation["interaction_effect"] - reference[("xm", "y")])
    checks = {
        "worker_payload_invariant": True,
        "bootstrap_original_matches_moderation_delta": original_delta,
        "independent_reference_delta": reference_delta,
        "simple_slope_reference_max_delta": slope_delta,
        "bootstrap_parameter_present": True,
        "bca_parameter_present": True,
        "permutation_parameter_present": True,
        "bootstrap_p_value_two_sided": percentile["p_value_two_sided"],
        "permutation_p_value_two_sided": permutation["p_value_two_sided"],
        "bca_lower": bca["lower"],
        "bca_upper": bca["upper"],
        "bootstrap_usable_replicates": one_worker["payload"]["bootstrap"]["usable_replicates"],
    }
    passed = (
        checks["bootstrap_original_matches_moderation_delta"] <= TOLERANCE
        and checks["independent_reference_delta"] <= TOLERANCE
        and checks["simple_slope_reference_max_delta"] <= TOLERANCE
        and checks["bootstrap_usable_replicates"] >= math.ceil(BOOTSTRAP_SAMPLES * 0.9)
        and percentile["standard_error"] > 0
        and percentile["lower"] < percentile["upper"]
        and bca["lower"] is not None
        and bca["upper"] is not None
        and permutation["permutations"] == PERMUTATION_SAMPLES
        and 0.0 <= permutation["p_value_two_sided"] <= 1.0
    )
    report = {
        "schema_version": 1,
        "kind": "pls_two_stage_moderation_inference_v1",
        "passed": passed,
        "bootstrap_samples": BOOTSTRAP_SAMPLES,
        "permutation_samples": PERMUTATION_SAMPLES,
        "tolerance": TOLERANCE,
        "checks": checks,
        "parameter": parameter_key("path", path_parts),
        "note": "Bounded inference integration and worker-invariance check for the generated moderation product path; not a full coverage qualification.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"bootstrap_p={checks['bootstrap_p_value_two_sided']:.3g} | "
        f"permutation_p={checks['permutation_p_value_two_sided']:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
