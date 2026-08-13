"""Fail-closed boundary and metamorphic checks for PLS-PM v1."""

from __future__ import annotations

from typing import Any

from pls_algorithm_v1_factory_common import (
    WORK_ROOT,
    analytic_payload,
    run_pls,
    write_csv,
    write_identity_report,
)
from pls_core_persistence_gate import archive_checksum_probe
from pls_core_simulation import Scenario, generate_scenario


SOURCE = "validation/pls_core_boundary_gate.py"
TOLERANCE = 1e-6


def _maximum(values: list[float]) -> float:
    return max(values, default=0.0)


def mapped_payload_errors(
    left: dict[str, Any],
    right: dict[str, Any],
    *,
    row_map: list[int] | None = None,
) -> dict[str, Any]:
    left_estimation = left["estimation"]
    right_estimation = right["estimation"]
    left_paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in left_estimation["paths"]
    }
    right_paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in right_estimation["paths"]
    }
    left_outer = {
        (row["construct"], row["indicator"]): (row["loading"], row["weight"])
        for row in left_estimation["outer_estimates"]
    }
    right_outer = {
        (row["construct"], row["indicator"]): (row["loading"], row["weight"])
        for row in right_estimation["outer_estimates"]
    }
    left_effects = {
        (row["source"], row["target"]): (
            row["direct"],
            row["indirect"],
            row["total"],
        )
        for row in left_estimation["effects"]
    }
    right_effects = {
        (row["source"], row["target"]): (
            row["direct"],
            row["indirect"],
            row["total"],
        )
        for row in right_estimation["effects"]
    }
    score_errors: list[float] = []
    for construct, left_values in left_estimation["construct_scores"].items():
        right_values = right_estimation["construct_scores"][construct]
        mapping = row_map if row_map is not None else list(range(len(left_values)))
        score_errors.extend(
            abs(left_values[source] - right_values[target])
            for target, source in enumerate(mapping)
        )
    maxima = {
        "paths": _maximum(
            [abs(left_paths[key] - right_paths[key]) for key in left_paths]
        ),
        "outer": _maximum(
            [
                abs(left_outer[key][position] - right_outer[key][position])
                for key in left_outer
                for position in (0, 1)
            ]
        ),
        "r_squared": _maximum(
            [
                abs(left_estimation["r_squared"][key] - right_estimation["r_squared"][key])
                for key in left_estimation["r_squared"]
            ]
        ),
        "effects": _maximum(
            [
                abs(left_effects[key][position] - right_effects[key][position])
                for key in left_effects
                for position in (0, 1, 2)
            ]
        ),
        "construct_scores": _maximum(score_errors),
    }
    return {
        "passed": (
            set(left_paths) == set(right_paths)
            and set(left_outer) == set(right_outer)
            and set(left_effects) == set(right_effects)
            and all(value <= TOLERANCE for value in maxima.values())
        ),
        "max_abs_errors": maxima,
    }


def run_boundaries() -> dict[str, Any]:
    scenario = Scenario(
        "boundary",
        180,
        "path",
        ("reflective", "reflective", "reflective"),
        (3, 3, 3),
        20_260_841,
        non_gaussian=True,
    )
    fixture = generate_scenario(scenario)
    baseline_csv = WORK_ROOT / "boundary_baseline.csv"
    write_csv(baseline_csv, fixture["variables"], fixture["rows"])
    baseline = run_pls(
        name="factory_boundary_baseline",
        csv_path=baseline_csv,
        constructs=fixture["constructs"],
        paths=fixture["paths"],
    )

    constant_rows = [list(row) for row in fixture["rows"]]
    for row in constant_rows:
        row[0] = 5.0
    constant_csv = WORK_ROOT / "boundary_constant.csv"
    write_csv(constant_csv, fixture["variables"], constant_rows)
    constant = run_pls(
        name="factory_boundary_constant",
        csv_path=constant_csv,
        constructs=fixture["constructs"],
        paths=fixture["paths"],
        expect_success=False,
    )
    constant_text = (
        constant["execution"]["stdout_tail"] + constant["execution"]["stderr_tail"]
    ).lower()
    data_pathology = {
        "passed": constant["passed"]
        and any(word in constant_text for word in ("constant", "variance", "scale")),
        "no_partial_result": constant["passed"],
        "typed_diagnostic": constant_text[-1000:],
    }

    cycle_paths = [*fixture["paths"], {"source": "y", "target": "x"}]
    cyclic = run_pls(
        name="factory_boundary_cycle",
        csv_path=baseline_csv,
        constructs=fixture["constructs"],
        paths=cycle_paths,
        expect_success=False,
    )
    cycle_text = (
        cyclic["execution"]["stdout_tail"] + cyclic["execution"]["stderr_tail"]
    ).lower()
    unsupported_scope = {
        "passed": cyclic["passed"]
        and any(word in cycle_text for word in ("cycle", "cyclic", "acyclic", "recursive")),
        "no_partial_result": cyclic["passed"],
        "typed_diagnostic": cycle_text[-1000:],
    }

    reversed_rows_csv = WORK_ROOT / "boundary_rows_reversed.csv"
    write_csv(reversed_rows_csv, fixture["variables"], list(reversed(fixture["rows"])))
    rows_reversed = run_pls(
        name="factory_boundary_rows_reversed",
        csv_path=reversed_rows_csv,
        constructs=fixture["constructs"],
        paths=fixture["paths"],
    )
    row_reorder = mapped_payload_errors(
        baseline,
        rows_reversed,
        row_map=list(reversed(range(len(fixture["rows"])))),
    )

    construct_order = [2, 0, 1]
    reordered_constructs = [fixture["constructs"][index] for index in construct_order]
    constructs_reordered = run_pls(
        name="factory_boundary_constructs_reordered",
        csv_path=baseline_csv,
        constructs=reordered_constructs,
        paths=list(reversed(fixture["paths"])),
    )
    construct_reorder = mapped_payload_errors(baseline, constructs_reordered)

    variable_order = list(reversed(range(len(fixture["variables"]))))
    reordered_variables = [fixture["variables"][index] for index in variable_order]
    reordered_rows = [
        [row[index] for index in variable_order] for row in fixture["rows"]
    ]
    reordered_indicator_constructs = [
        {**construct, "indicators": list(reversed(construct["indicators"]))}
        for construct in fixture["constructs"]
    ]
    indicators_reordered_csv = WORK_ROOT / "boundary_indicators_reordered.csv"
    write_csv(indicators_reordered_csv, reordered_variables, reordered_rows)
    indicators_reordered = run_pls(
        name="factory_boundary_indicators_reordered",
        csv_path=indicators_reordered_csv,
        constructs=reordered_indicator_constructs,
        paths=fixture["paths"],
    )
    indicator_reorder = mapped_payload_errors(baseline, indicators_reordered)
    metamorphic = {
        "passed": row_reorder["passed"]
        and construct_reorder["passed"]
        and indicator_reorder["passed"],
        "row_reorder": row_reorder,
        "construct_and_path_reorder": construct_reorder,
        "indicator_reorder": indicator_reorder,
    }

    repeated = run_pls(
        name="factory_boundary_repeat",
        csv_path=baseline_csv,
        constructs=fixture["constructs"],
        paths=fixture["paths"],
    )
    first_payload = analytic_payload(baseline)
    second_payload = analytic_payload(repeated)
    determinism = {
        "passed": first_payload == second_payload,
        "analytic_payloads_exactly_equal": first_payload == second_payload,
        "first_output_sha256": baseline["output_sha256"],
        "second_output_sha256": repeated["output_sha256"],
    }

    tamper = archive_checksum_probe()
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
            "output": baseline["output"],
            "output_sha256": baseline["output_sha256"],
        },
        "categories": categories,
    }


def main() -> int:
    detail = run_boundaries()
    path = write_identity_report(
        "boundary_report",
        passed=detail["passed"],
        checks=detail,
        extras=[
            SOURCE,
            "validation/pls_core_persistence_gate.py",
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-project/src/lib.rs",
            "crates/qpls-project/src/archive_integrity.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
        ],
    )
    print(f"wrote {path} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
