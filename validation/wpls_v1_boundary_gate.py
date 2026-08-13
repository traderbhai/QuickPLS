"""Fail-closed WPLS boundary, metamorphic, and determinism evidence."""

from __future__ import annotations

from typing import Any

from wpls_v1_factory_common import WORK_ROOT, construct, run_model, write_csv, write_identity_report
from wpls_v1_persistence_gate import archive_weight_metadata_probe
from wpls_v1_simulation import Scenario, generate


SOURCE = "validation/wpls_v1_boundary_gate.py"
TOLERANCE = 1e-6


def _table(estimation: dict[str, Any]) -> dict[str, Any]:
    return {
        "paths": {(row["source"], row["target"]): row["coefficient"] for row in estimation["paths"]},
        "outer": {
            (row["construct"], row["indicator"]): (row["loading"], row["weight"])
            for row in estimation["outer_estimates"]
        },
        "r_squared": estimation["r_squared"],
        "effects": {
            (row["source"], row["target"]): (row["direct"], row["indirect"], row["total"])
            for row in estimation["effects"]
        },
        "scores": estimation["construct_scores"],
    }


def _mapped_errors(
    left: dict[str, Any], right: dict[str, Any], *, row_map: list[int] | None = None
) -> dict[str, Any]:
    a = _table(left["estimation"])
    b = _table(right["estimation"])
    values: dict[str, list[float]] = {
        "paths": [abs(a["paths"][key] - b["paths"][key]) for key in a["paths"]],
        "outer": [
            abs(a["outer"][key][position] - b["outer"][key][position])
            for key in a["outer"]
            for position in (0, 1)
        ],
        "r_squared": [abs(a["r_squared"][key] - b["r_squared"][key]) for key in a["r_squared"]],
        "effects": [
            abs(a["effects"][key][position] - b["effects"][key][position])
            for key in a["effects"]
            for position in (0, 1, 2)
        ],
        "construct_scores": [],
    }
    for construct_id, left_scores in a["scores"].items():
        right_scores = b["scores"][construct_id]
        mapping = row_map if row_map is not None else list(range(len(left_scores)))
        values["construct_scores"].extend(
            abs(left_scores[source] - right_scores[target])
            for target, source in enumerate(mapping)
        )
    maxima = {key: max(errors) if errors else 0.0 for key, errors in values.items()}
    return {
        "passed": (
            set(a["paths"]) == set(b["paths"])
            and set(a["outer"]) == set(b["outer"])
            and set(a["effects"]) == set(b["effects"])
            and all(value <= TOLERANCE for value in maxima.values())
        ),
        "max_abs_errors": maxima,
    }


def _write_fixture(path: Any, rows: list[dict[str, float | None]]) -> None:
    names = ["x1", "x2", "y1", "y2", "case_wt"]
    write_csv(path, names, [[row[name] for name in names] for row in rows])


def _run(name: str, csv_path: Any, **kwargs: Any) -> dict[str, Any]:
    return run_model(
        name=name,
        csv_path=csv_path,
        constructs=[construct("x", ["x1", "x2"]), construct("y", ["y1", "y2"])],
        paths=[{"source": "x", "target": "y"}],
        **kwargs,
    )


def run_boundaries() -> dict[str, Any]:
    rows, _, _ = generate(Scenario("boundary", 160, 20_260_861, "path", "moderate"))
    baseline_csv = WORK_ROOT / "boundary_baseline.csv"
    _write_fixture(baseline_csv, rows)
    baseline = _run("factory_boundary_baseline", baseline_csv)

    invalid_runs: dict[str, Any] = {}
    for label, value in (("zero", 0.0), ("negative", -0.75)):
        invalid_rows = [dict(row) for row in rows]
        invalid_rows[5]["case_wt"] = value
        path = WORK_ROOT / f"boundary_weight_{label}.csv"
        _write_fixture(path, invalid_rows)
        run = _run(f"factory_boundary_weight_{label}", path, expect_success=False)
        diagnostic = (run["execution"]["stdout_tail"] + run["execution"]["stderr_tail"]).lower()
        invalid_runs[label] = {
            "passed": run["passed"] and "positive" in diagnostic and "finite" in diagnostic,
            "no_partial_result": run["passed"],
            "diagnostic": diagnostic[-1200:],
        }
    data_pathology = {
        "passed": all(row["passed"] for row in invalid_runs.values()),
        "invalid_weights": invalid_runs,
    }

    missing_column = _run(
        "factory_boundary_missing_weight_column",
        baseline_csv,
        case_weight_column=None,
        expect_success=False,
    )
    pca = _run(
        "factory_boundary_pca",
        baseline_csv,
        weighting_scheme="pca",
        expect_success=False,
    )
    formative = run_model(
        name="factory_boundary_formative",
        csv_path=baseline_csv,
        constructs=[construct("x", ["x1", "x2"], mode="formative"), construct("y", ["y1", "y2"])],
        paths=[{"source": "x", "target": "y"}],
        expect_success=False,
    )
    resampling = _run(
        "factory_boundary_resampling",
        baseline_csv,
        bootstrap_samples=100,
        expect_success=False,
    )
    unsupported_rows = {
        "missing_weight_column": missing_column,
        "pca_weighting": pca,
        "formative_measurement": formative,
        "resampling": resampling,
    }
    unsupported_scope = {
        "passed": all(row["passed"] for row in unsupported_rows.values()),
        "blocked_combinations": {
            key: {
                "passed": row["passed"],
                "diagnostic": (row["execution"]["stdout_tail"] + row["execution"]["stderr_tail"])[-1200:],
            }
            for key, row in unsupported_rows.items()
        },
    }

    scaled_rows = [{**row, "case_wt": float(row["case_wt"]) * 11.0} for row in rows]
    scaled_csv = WORK_ROOT / "boundary_weights_scaled.csv"
    _write_fixture(scaled_csv, scaled_rows)
    scaled = _run("factory_boundary_weights_scaled", scaled_csv)
    scale_invariance = _mapped_errors(baseline, scaled)
    scale_invariance["effective_sample_size_error"] = abs(
        baseline["wpls"]["effective_sample_size"] - scaled["wpls"]["effective_sample_size"]
    )
    scale_invariance["passed"] = (
        scale_invariance["passed"] and scale_invariance["effective_sample_size_error"] <= TOLERANCE
    )

    uniform_rows = [{**row, "case_wt": 3.0} for row in rows]
    uniform_csv = WORK_ROOT / "boundary_weights_uniform.csv"
    _write_fixture(uniform_csv, uniform_rows)
    uniform_wpls = _run("factory_boundary_uniform_wpls", uniform_csv)
    ordinary_pls = _run(
        "factory_boundary_uniform_ordinary",
        uniform_csv,
        method="pls_pm",
        case_weight_column=None,
    )
    uniform_reduction = _mapped_errors(uniform_wpls, ordinary_pls)

    reversed_csv = WORK_ROOT / "boundary_rows_reversed.csv"
    _write_fixture(reversed_csv, list(reversed(rows)))
    reversed_run = _run("factory_boundary_rows_reversed", reversed_csv)
    row_reorder = _mapped_errors(
        baseline,
        reversed_run,
        row_map=list(reversed(range(len(rows)))),
    )
    metamorphic = {
        "passed": scale_invariance["passed"] and uniform_reduction["passed"] and row_reorder["passed"],
        "positive_weight_scale_invariance": scale_invariance,
        "uniform_weights_reduce_to_ordinary_pls": uniform_reduction,
        "consistent_row_reorder": row_reorder,
    }

    repeated = _run("factory_boundary_repeat", baseline_csv)
    first = _table(baseline["estimation"])
    second = _table(repeated["estimation"])
    determinism = {
        "passed": first == second and baseline["wpls"] == repeated["wpls"],
        "analytic_payloads_exactly_equal": first == second,
        "weight_metadata_exactly_equal": baseline["wpls"] == repeated["wpls"],
        "first_output_sha256": baseline["output_sha256"],
        "second_output_sha256": repeated["output_sha256"],
    }

    tamper = archive_weight_metadata_probe()
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
        "baseline": {"output": baseline["output"], "output_sha256": baseline["output_sha256"]},
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
            "validation/wpls_v1_simulation.py",
            "validation/wpls_v1_persistence_gate.py",
            "crates/qpls-core/src/validation.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-project/src/lib.rs",
        ],
    )
    print(f"wrote {path} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
