"""Fail-closed boundary, metamorphic, and tamper checks for CTA-PLS v1."""

from __future__ import annotations

import math
from typing import Any

from cta_pls_simulation import generated_rows
from cta_pls_v1_factory_common import (
    WORK_ROOT,
    analytic_payload,
    construct,
    run_command,
    run_cta_pls,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/cta_pls_boundary_gate.py"
PROJECT_SOURCE = "crates/qpls-project/src/lib.rs"
TOLERANCE = 1e-6


def _partition(left: str, right: str, other_left: str, other_right: str) -> tuple[tuple[str, str], tuple[str, str]]:
    return tuple(sorted((tuple(sorted((left, right))), tuple(sorted((other_left, other_right))))))  # type: ignore[return-value]


def canonical_tetrads(run: dict[str, Any]) -> dict[tuple[Any, ...], float]:
    values: dict[tuple[Any, ...], float] = {}
    for row in run["cta"]["estimates"]:
        a, b, c, d = (
            row["indicator_a"],
            row["indicator_b"],
            row["indicator_c"],
            row["indicator_d"],
        )
        partitions = {
            "ab": _partition(a, b, c, d),
            "ac": _partition(a, c, b, d),
            "ad": _partition(a, d, b, c),
        }
        pairing = row["pairing"]
        if pairing == "ab_cd_minus_ac_bd":
            left, right = partitions["ab"], partitions["ac"]
        elif pairing == "ac_bd_minus_ad_bc":
            left, right = partitions["ac"], partitions["ad"]
        elif pairing == "ad_bc_minus_ab_cd":
            left, right = partitions["ad"], partitions["ab"]
        else:
            raise ValueError(f"unknown CTA pairing: {pairing}")
        value = float(row["tetrad"])
        if right < left:
            left, right = right, left
            value = -value
        key = (row["construct"], tuple(sorted((a, b, c, d))), left, right)
        if key in values:
            raise ValueError(f"duplicate canonical CTA key: {key}")
        values[key] = value
    return values


def compare_canonical(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    left_values = canonical_tetrads(left)
    right_values = canonical_tetrads(right)
    exact_membership = set(left_values) == set(right_values)
    errors = [
        abs(left_values[key] - right_values[key])
        for key in set(left_values) & set(right_values)
    ]
    maximum = max(errors, default=math.inf)
    return {
        "passed": exact_membership and bool(left_values) and maximum <= TOLERANCE,
        "exact_canonical_membership": exact_membership,
        "canonical_tetrad_count": len(left_values),
        "max_abs_error": maximum,
        "missing_from_right": [repr(key) for key in sorted(set(left_values) - set(right_values))],
        "unexpected_in_right": [repr(key) for key in sorted(set(right_values) - set(left_values))],
    }


def run_boundaries() -> dict[str, Any]:
    variables, rows = generated_rows(
        seed=20_260_941,
        sample_size=96,
        x_count=5,
        y_count=2,
        non_gaussian=True,
        correlated_error=True,
        missing=False,
    )
    x_indicators = variables[:5]
    y_indicators = variables[5:]
    model = [construct("x", x_indicators), construct("y", y_indicators)]
    paths = [{"source": "x", "target": "y"}]
    baseline_path = WORK_ROOT / "boundary_baseline.csv"
    write_csv(baseline_path, variables, rows)
    baseline = run_cta_pls(
        name="factory_boundary_baseline",
        csv_path=baseline_path,
        constructs=model,
        paths=paths,
    )

    constant_path = WORK_ROOT / "boundary_constant.csv"
    constant_rows = [list(row[:4]) + [7.0] + list(row[5:]) for row in rows]
    write_csv(constant_path, variables, constant_rows)
    constant = run_cta_pls(
        name="factory_boundary_constant",
        csv_path=constant_path,
        constructs=model,
        paths=paths,
        expect_success=False,
    )
    constant_text = (
        constant["execution"]["stdout_tail"] + constant["execution"]["stderr_tail"]
    ).lower()
    data_pathology = {
        "passed": constant["passed"] and ("constant" in constant_text or "variance" in constant_text),
        "no_partial_result": constant["passed"],
        "typed_diagnostic": "constant" in constant_text or "variance" in constant_text,
        "execution": constant["execution"],
    }

    insufficient = run_cta_pls(
        name="factory_boundary_insufficient_block",
        csv_path=baseline_path,
        constructs=[construct("x", x_indicators[:3]), construct("y", y_indicators)],
        paths=paths,
        expect_success=False,
    )
    insufficient_text = (
        insufficient["execution"]["stdout_tail"] + insufficient["execution"]["stderr_tail"]
    ).lower()
    pca = run_cta_pls(
        name="factory_boundary_pca_weighting",
        csv_path=baseline_path,
        constructs=model,
        paths=paths,
        weighting_scheme="pca",
        expect_success=False,
    )
    pca_text = pca["execution"]["stdout_tail"] + pca["execution"]["stderr_tail"]
    unsupported_scope = {
        "passed": (
            insufficient["passed"]
            and "cta_pls.tetrad_block_required" in insufficient_text
            and pca["passed"]
            and "cta_pls.pca_unsupported" in pca_text
        ),
        "insufficient_block_rejected_without_result": insufficient["passed"],
        "insufficient_block_code": "cta_pls.tetrad_block_required" in insufficient_text,
        "pca_weighting_rejected_without_result": pca["passed"],
        "pca_weighting_code": "cta_pls.pca_unsupported" in pca_text,
        "executions": [insufficient["execution"], pca["execution"]],
    }

    reversed_path = WORK_ROOT / "boundary_rows_reversed.csv"
    write_csv(reversed_path, variables, list(reversed(rows)))
    row_reordered = run_cta_pls(
        name="factory_boundary_rows_reversed",
        csv_path=reversed_path,
        constructs=model,
        paths=paths,
    )
    row_comparison = compare_canonical(baseline, row_reordered)

    indicator_reordered = run_cta_pls(
        name="factory_boundary_indicators_reversed",
        csv_path=baseline_path,
        constructs=[construct("x", list(reversed(x_indicators))), construct("y", y_indicators)],
        paths=paths,
    )
    indicator_comparison = compare_canonical(baseline, indicator_reordered)
    metamorphic = {
        "passed": row_comparison["passed"] and indicator_comparison["passed"],
        "row_reorder": row_comparison,
        "indicator_reorder": indicator_comparison,
    }

    repeated = run_cta_pls(
        name="factory_boundary_repeat",
        csv_path=baseline_path,
        constructs=model,
        paths=paths,
    )
    baseline_payload = analytic_payload(baseline)
    repeated_payload = analytic_payload(repeated)
    determinism = {
        "passed": baseline_payload == repeated_payload,
        "analytical_payloads_exactly_equal": baseline_payload == repeated_payload,
        "first_output_sha256": baseline["output_sha256"],
        "second_output_sha256": repeated["output_sha256"],
    }

    cargo, cargo_execution = run_command(
        [
            "cargo",
            "test",
            "-p",
            "qpls-project",
            "tests::runner_generated_cta_pls_appends_round_trips_and_rejects_contract_tampering",
            "--",
        ],
        timeout=900,
    )
    cargo_text = cargo.stdout + cargo.stderr
    tamper = {
        "passed": cargo.returncode == 0 and "1 passed" in cargo_text and "0 failed" in cargo_text,
        "method_scoped_archive_test": True,
        "mutations": [
            "method_version",
            "unknown_pairing",
            "duplicate_pairing",
            "absolute_value",
            "construct_maximum",
            "warning_contract",
            "resampling_identity",
        ],
        "execution": cargo_execution,
    }

    categories = {
        "data_pathology": data_pathology,
        "unsupported_scope": unsupported_scope,
        "metamorphic": metamorphic,
        "determinism": determinism,
        "tamper": tamper,
    }
    return {
        "passed": baseline["passed"] and all(row["passed"] for row in categories.values()),
        "tolerance": TOLERANCE,
        "baseline_identity": {
            "passed": baseline["identity_passed"],
            "pairing_count": len(baseline["cta"]["estimates"]),
            "output": baseline["output"],
            "output_sha256": baseline["output_sha256"],
        },
        "categories": categories,
    }


def main() -> int:
    detail = run_boundaries()
    report = write_identity_report(
        "boundary_report",
        passed=detail["passed"],
        checks=detail,
        extras=[
            SOURCE,
            PROJECT_SOURCE,
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-core/src/validation.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
        ],
    )
    print(f"wrote {report} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
