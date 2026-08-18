from __future__ import annotations

import ast
import json
from dataclasses import replace
from pathlib import Path

import numpy as np
import pytest

import pls_score_execution_v2_oracle as oracle


WORK_REPORT = (
    Path(__file__).resolve().parent
    / "results"
    / "method_factory"
    / "pls_algorithm_v1"
    / "work"
    / "pls_score_execution_v2_independent_oracle_work.json"
)


def _individual_fixture() -> tuple[dict[str, object], oracle.OracleInitialization]:
    fixture = oracle.deterministic_fixture()
    values = {
        (block.construct_id, indicator): value
        for block, block_values in zip(
            fixture["blocks"],
            ((-1.0, -0.35, 0.20), (0.15, 1.0, -0.30), (0.30, -0.10, 1.0)),
        )
        for indicator, value in zip(block.indicator_ids, block_values)
    }
    return fixture, oracle._individual_initialization(fixture, values)  # noqa: SLF001


def test_work_checks_are_deterministic_and_explicitly_non_promotional() -> None:
    first = oracle.run_work_checks()
    second = oracle.run_work_checks()

    assert first == second
    assert first["passed"]
    assert first["work_evidence_only"]
    assert not first["qualification_ready"]
    assert not first["promotion_requested"]
    assert not first["product_comparison_performed"]
    assert not first["receipt_attached"]
    assert not first["registry_or_manifest_mutation_requested"]
    assert all(first["checks"].values())
    assert all(first["typed_boundaries"].values())


def test_standard_and_individual_initialization_have_distinct_deterministic_trajectories() -> (
    None
):
    fixture, initialization = _individual_fixture()
    standard = oracle.estimate_score_execution_v2(**fixture)
    individual = oracle.estimate_score_execution_v2(
        **fixture, initialization=initialization
    )

    assert standard == oracle.estimate_score_execution_v2(**fixture)
    assert individual == oracle.estimate_score_execution_v2(
        **fixture, initialization=initialization
    )
    assert standard["converged"] and individual["converged"]
    assert standard["initial_state_sha256"] != individual["initial_state_sha256"]
    assert standard["iteration_trace_sha256"] != individual["iteration_trace_sha256"]
    assert (
        standard["iteration_accounting"]["performed_iterations"]
        != individual["iteration_accounting"]["performed_iterations"]
    )

    standard_x = next(
        block for block in standard["blocks"] if block["construct_id"] == "x"
    )
    individual_x = next(
        block for block in individual["blocks"] if block["construct_id"] == "x"
    )
    assert set(standard_x["requested_weights"].values()) == {1.0}
    assert individual_x["requested_weights"] == {
        "x1": -1.0,
        "x2": -0.35,
        "x3": 0.20,
    }
    assert individual_x["effective_initial_weights"]["x1"] < 0.0
    assert individual_x["effective_initial_weights"]["x2"] < 0.0
    assert individual_x["effective_initial_weights"]["x3"] > 0.0

    report = oracle.run_work_checks()
    initialization_case = report["initialization_cases"]
    assert (
        initialization_case["converged_solution_max_abs_difference"]
        <= (initialization_case["converged_solution_tolerance"])
    )


def test_pathological_individual_start_fails_where_standard_start_converges() -> None:
    fixture, pathological = oracle._pathological_initialization_fixture()  # noqa: SLF001
    assert oracle.estimate_score_execution_v2(**fixture)["converged"]

    with pytest.raises(oracle.ScoreExecutionOracleFailure) as raised:
        oracle.estimate_score_execution_v2(
            **fixture,
            initialization=pathological,
        )
    assert raised.value.code == "pls_score_execution_v2_zero_variance_requested_score"


def test_fixed_unit_and_custom_scores_use_unit_variance_and_zero_iterations() -> None:
    fixture = oracle._fixed_only_fixture()  # noqa: SLF001
    result = oracle.estimate_score_execution_v2(**fixture)

    assert result["method_version"] == "pls_score_execution_v2"
    assert result["contract_version"] == "pls_score_execution_v2"
    assert result["iteration_accounting"]["performed_iterations"] == 0
    assert result["iteration_accounting"]["estimated_block_updates"] == 0
    assert result["iteration_accounting"]["estimated_block_count"] == 0
    assert result["iteration_accounting"]["fixed_block_count"] == 2

    for block in result["blocks"]:
        assert block["normalization"] == "unit_variance"
        assert block["effective_initial_weights"] == block["final_weights"]

    unit = next(block for block in result["blocks"] if block["scoring"] == "unit")
    custom = next(block for block in result["blocks"] if block["scoring"] == "custom")
    assert set(unit["requested_weights"].values()) == {1.0}
    assert custom["requested_weights"] == {"y1": -0.25, "y2": 0.75, "y3": 0.40}
    assert custom["effective_initial_weights"]["y1"] < 0.0
    assert custom["effective_initial_weights"]["y2"] > 0.0

    raw = np.asarray(fixture["rows"], dtype=float)
    positions = {name: index for index, name in enumerate(fixture["variables"])}
    standardized = (raw - raw.mean(axis=0)) / raw.std(axis=0, ddof=1)
    x_matrix = standardized[:, [positions[name] for name in ("x1", "x2", "x3")]]
    expected_x = x_matrix.sum(axis=1)
    expected_x = (expected_x - expected_x.mean()) / expected_x.std(ddof=1)
    observed_x = np.asarray(
        [result["construct_scores"]["x"][row_id] for row_id in fixture["row_ids"]]
    )
    np.testing.assert_allclose(observed_x, expected_x, atol=2e-14, rtol=0.0)


def test_mixed_scoring_updates_only_estimated_blocks() -> None:
    result = oracle.estimate_score_execution_v2(**oracle._mixed_fixture())  # noqa: SLF001
    accounting = result["iteration_accounting"]

    assert accounting["performed_iterations"] > 0
    assert accounting["estimated_block_count"] == 1
    assert accounting["fixed_block_count"] == 2
    assert accounting["estimated_block_updates"] == accounting["performed_iterations"]
    for block in result["blocks"]:
        if block["scoring"] == "estimated":
            assert block["effective_initial_weights"] != block["final_weights"]
        else:
            assert block["effective_initial_weights"] == block["final_weights"]


@pytest.mark.parametrize("normalization", ["none", "sum_to_one"])
@pytest.mark.parametrize("scoring", ["unit", "custom"])
def test_unsupported_fixed_normalizations_fail_with_one_typed_code(
    normalization: oracle.Normalization,
    scoring: oracle.BlockScoring,
) -> None:
    block = oracle.OracleScoreBlock(
        "fixed",
        ("f1", "f2"),
        scoring=scoring,
        normalization=normalization,
        custom_weights=(("f1", -0.25), ("f2", 0.75)) if scoring == "custom" else (),
    )
    with pytest.raises(oracle.ScoreExecutionOracleFailure) as raised:
        oracle.estimate_score_execution_v2(
            [[1.0, 2.0], [2.0, 4.0], [4.0, 7.0], [7.0, 9.0]],
            ["r1", "r2", "r3", "r4"],
            ["f1", "f2"],
            [block],
            [],
        )
    assert raised.value.code == "pls_score_execution_v2_fixed_normalization_unsupported"


@pytest.mark.parametrize(
    ("settings", "code"),
    [
        (
            oracle.OracleExecutionSettings(bootstrap_samples=1),
            "pls_score_execution_v2_resampling_unsupported",
        ),
        (
            oracle.OracleExecutionSettings(studentized_inner_samples=1),
            "pls_score_execution_v2_resampling_unsupported",
        ),
        (
            oracle.OracleExecutionSettings(permutation_samples=1),
            "pls_score_execution_v2_resampling_unsupported",
        ),
        (
            oracle.OracleExecutionSettings(preprocessing="mean_centered"),
            "pls_score_execution_v2_result_location_unsupported",
        ),
        (
            oracle.OracleExecutionSettings(preprocessing="unstandardized"),
            "pls_score_execution_v2_result_location_unsupported",
        ),
        (
            oracle.OracleExecutionSettings(case_weights_requested=True),
            "pls_score_execution_v2_case_weights_unsupported",
        ),
        (
            oracle.OracleExecutionSettings(higher_order_requested=True),
            "pls_score_execution_v2_higher_order_unsupported",
        ),
        (
            oracle.OracleExecutionSettings(interaction_requested=True),
            "pls_score_execution_v2_interaction_unsupported",
        ),
    ],
)
def test_scope_drift_fails_closed(
    settings: oracle.OracleExecutionSettings, code: str
) -> None:
    with pytest.raises(oracle.ScoreExecutionOracleFailure) as raised:
        oracle.estimate_score_execution_v2(
            **oracle._fixed_only_fixture(),  # noqa: SLF001
            settings=settings,
        )
    assert raised.value.code == code


def test_individual_initialization_is_stable_id_exact_and_excludes_fixed_blocks() -> (
    None
):
    fixture, initialization = _individual_fixture()
    reversed_rows = replace(
        initialization, weights=tuple(reversed(initialization.weights))
    )
    with pytest.raises(oracle.ScoreExecutionOracleFailure) as raised:
        oracle.estimate_score_execution_v2(
            **fixture,
            initialization=reversed_rows,
        )
    assert raised.value.code == "pls_score_execution_v2_individual_order_invalid"

    mixed = oracle._mixed_fixture()  # noqa: SLF001
    invalid = oracle.OracleInitialization(
        "individual",
        tuple(
            sorted(
                (
                    oracle.OracleInitialWeight(block.construct_id, indicator, 1.0)
                    for block in mixed["blocks"]
                    for indicator in block.indicator_ids
                ),
                key=lambda row: (row.construct_id, row.indicator_id),
            )
        ),
    )
    with pytest.raises(oracle.ScoreExecutionOracleFailure) as raised:
        oracle.estimate_score_execution_v2(**mixed, initialization=invalid)
    assert raised.value.code == "pls_score_execution_v2_individual_coverage"


def test_id_order_row_and_positive_affine_metamorphics_are_bounded() -> None:
    report = oracle.run_work_checks()
    differences = report["metamorphic_max_abs_differences"]

    assert set(differences) == {
        "stable_id_renaming",
        "model_declaration_order",
        "variable_declaration_order",
        "stable_row_order",
        "positive_affine_rescaling",
    }
    assert max(differences.values()) <= report["metamorphic_tolerance"]


def test_oracle_import_graph_is_independent_of_quickpls_product_code() -> None:
    source_path = Path(oracle.__file__).resolve()
    syntax = ast.parse(source_path.read_text(encoding="utf-8"))
    imported_roots: set[str] = set()
    for node in ast.walk(syntax):
        if isinstance(node, ast.Import):
            imported_roots.update(alias.name.split(".")[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module is not None:
            imported_roots.add(node.module.split(".")[0])

    assert imported_roots <= {
        "__future__",
        "argparse",
        "hashlib",
        "json",
        "math",
        "dataclasses",
        "pathlib",
        "typing",
        "numpy",
    }
    assert not any(root.startswith("qpls") for root in imported_roots)


def test_work_report_is_strict_json_and_remains_unqualified(tmp_path: Path) -> None:
    path = tmp_path / "pls_score_execution_v2_work.json"
    expected = oracle.write_work_report(path)
    observed = json.loads(
        path.read_text(encoding="utf-8"),
        parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)),
    )

    assert observed == expected
    assert not observed["qualification_spec_v2_alignment"]["admissible_receipt"]
    assert observed["qualification_spec_v2_alignment"]["candidate_oracle_kind"] == (
        "independent_implementation"
    )
    assert observed["qualification_spec_v2_alignment"]["runtime_policy"] == (
        "development_validation_only"
    )


def test_repository_work_report_is_fresh_and_cannot_claim_qualification() -> None:
    observed = json.loads(
        WORK_REPORT.read_text(encoding="utf-8"),
        parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)),
    )

    assert observed == oracle.run_work_checks()
    assert observed["work_evidence_only"]
    assert not observed["qualification_ready"]
    assert not observed["product_comparison_performed"]
    assert not observed["receipt_attached"]
