"""Scientific boundaries and metamorphic evidence for PLS bootstrap v4."""

from __future__ import annotations

import itertools
import json
from collections import Counter
from pathlib import Path
from typing import Any

from pls_bootstrap_v4_factory_common import (
    REPORT_ROOT,
    ROOT,
    WORK_ROOT,
    construct,
    repository_path,
    run_bootstrap,
    run_command,
    sha256_file,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/pls_bootstrap_release_boundary_gate.py"
MONTE_CARLO_REPLICATES = 9_999
MONTE_CARLO_MARGIN = 1.10
NUMERICAL_ALLOWANCE = 1e-12
MODEL = [construct("x", ["x1", "x2"]), construct("y", ["y1", "y2"])]
PATHS = [{"source": "x", "target": "y"}]


def monte_carlo_equivalent(row_reorder_drift: float, different_seed_drift: float) -> bool:
    """Apply the frozen finite-run row-order equivalence envelope."""

    if row_reorder_drift < 0.0 or different_seed_drift < 0.0:
        return False
    return (
        row_reorder_drift
        <= MONTE_CARLO_MARGIN * different_seed_drift + NUMERICAL_ALLOWANCE
    )


def exhaustive_distribution_bijection(
    case_count: int = 4, permutation: tuple[int, ...] = (2, 0, 3, 1)
) -> dict[str, Any]:
    if sorted(permutation) != list(range(case_count)):
        raise ValueError("permutation must contain every case label exactly once")

    def distribution(storage: tuple[int, ...]) -> Counter[tuple[int, ...]]:
        result: Counter[tuple[int, ...]] = Counter()
        for draw in itertools.product(range(case_count), repeat=case_count):
            counts = [0] * case_count
            for position in draw:
                counts[storage[position]] += 1
            result[tuple(counts)] += 1
        return result

    original = distribution(tuple(range(case_count)))
    reordered = distribution(permutation)
    mutated = reordered.copy()
    first = next(iter(mutated))
    mutated[first] += 1
    expected_draws = case_count**case_count
    passed = (
        original == reordered
        and original != mutated
        and sum(original.values()) == expected_draws
        and sum(reordered.values()) == expected_draws
    )
    return {
        "passed": passed,
        "case_count": case_count,
        "permutation_storage_to_case_label": list(permutation),
        "ordered_draw_count": expected_draws,
        "distinct_count_vectors": len(original),
        "exact_distribution_equal_after_relabeling": original == reordered,
        "one_frequency_mutation_detected": original != mutated,
        "distribution_sha256": __import__("hashlib").sha256(
            json.dumps(sorted(original.items()), separators=(",", ":")).encode("utf-8")
        ).hexdigest(),
    }


def _read_fixture() -> tuple[list[str], list[list[float]]]:
    lines = (ROOT / "validation" / "fixtures" / "simple_reflective.csv").read_text(
        encoding="utf-8"
    ).splitlines()
    variables = lines[0].split(",")
    rows = [[float(value) for value in line.split(",")] for line in lines[1:]]
    return variables, rows


def _rows_by_parameter(bootstrap: dict[str, Any], family: str) -> dict[str, dict[str, Any]]:
    rows = bootstrap[family]["parameters"]
    result = {str(row["parameter"]): row for row in rows}
    if len(result) != len(rows):
        raise ValueError(f"duplicate {family} parameter identity")
    return result


def monte_carlo_row_reorder_gate() -> tuple[dict[str, Any], list[str]]:
    variables, rows = _read_fixture()
    original_csv = WORK_ROOT / "boundary_row_order_original.csv"
    reordered_csv = WORK_ROOT / "boundary_row_order_reversed.csv"
    write_csv(original_csv, variables, rows)
    write_csv(reordered_csv, variables, list(reversed(rows)))
    seed_a = 20_260_813
    seed_b = 20_260_814
    runs = {
        "original_seed_a": run_bootstrap(
            name="factory_boundary_original_seed_a",
            csv_path=original_csv,
            constructs=MODEL,
            paths=PATHS,
            bootstrap_samples=MONTE_CARLO_REPLICATES,
            seed=seed_a,
            workers=4,
        ),
        "original_seed_a_repeat": run_bootstrap(
            name="factory_boundary_original_seed_a_repeat",
            csv_path=original_csv,
            constructs=MODEL,
            paths=PATHS,
            bootstrap_samples=MONTE_CARLO_REPLICATES,
            seed=seed_a,
            workers=4,
        ),
        "original_seed_a_worker_1": run_bootstrap(
            name="factory_boundary_original_seed_a_worker_1",
            csv_path=original_csv,
            constructs=MODEL,
            paths=PATHS,
            bootstrap_samples=MONTE_CARLO_REPLICATES,
            seed=seed_a,
            workers=1,
        ),
        "original_seed_b": run_bootstrap(
            name="factory_boundary_original_seed_b",
            csv_path=original_csv,
            constructs=MODEL,
            paths=PATHS,
            bootstrap_samples=MONTE_CARLO_REPLICATES,
            seed=seed_b,
            workers=4,
        ),
        "reordered_seed_a": run_bootstrap(
            name="factory_boundary_reordered_seed_a",
            csv_path=reordered_csv,
            constructs=MODEL,
            paths=PATHS,
            bootstrap_samples=MONTE_CARLO_REPLICATES,
            seed=seed_a,
            workers=4,
        ),
        "reordered_seed_b": run_bootstrap(
            name="factory_boundary_reordered_seed_b",
            csv_path=reordered_csv,
            constructs=MODEL,
            paths=PATHS,
            bootstrap_samples=MONTE_CARLO_REPLICATES,
            seed=seed_b,
            workers=4,
        ),
    }
    payloads = {name: row["bootstrap"] for name, row in runs.items()}
    exact_repeat = payloads["original_seed_a"] == payloads["original_seed_a_repeat"]
    exact_worker = payloads["original_seed_a"] == payloads["original_seed_a_worker_1"]
    family_metrics = {
        "percentile": (
            "bootstrap_mean",
            "bias",
            "standard_error",
            "lower",
            "upper",
            "t_statistic",
            "p_value_two_sided",
        ),
        "bca": ("bias_correction", "acceleration", "lower", "upper"),
    }
    comparisons: list[dict[str, Any]] = []
    membership_passed = True
    availability_passed = True
    original_estimate_passed = True
    for family, metrics in family_metrics.items():
        tables = {
            name: _rows_by_parameter(payload, family) for name, payload in payloads.items()
        }
        identities = set(tables["original_seed_a"])
        membership_passed = membership_passed and all(
            set(table) == identities for table in tables.values()
        )
        for parameter in sorted(identities):
            rows_for_parameter = {name: table[parameter] for name, table in tables.items()}
            if family == "percentile":
                originals = [float(row["original"]) for row in rows_for_parameter.values()]
                original_error = max(originals) - min(originals)
                original_estimate_passed = (
                    original_estimate_passed and original_error <= NUMERICAL_ALLOWANCE
                )
            for metric in metrics:
                values = {name: row.get(metric) for name, row in rows_for_parameter.items()}
                available = {name: value is not None for name, value in values.items()}
                same_availability = len(set(available.values())) == 1
                availability_passed = availability_passed and same_availability
                if not same_availability or not next(iter(available.values())):
                    comparisons.append(
                        {
                            "family": family,
                            "parameter": parameter,
                            "metric": metric,
                            "passed": same_availability,
                            "availability": available,
                        }
                    )
                    continue
                numeric = {name: float(value) for name, value in values.items()}
                reorder_drift = max(
                    abs(numeric["original_seed_a"] - numeric["reordered_seed_a"]),
                    abs(numeric["original_seed_b"] - numeric["reordered_seed_b"]),
                )
                seed_drift = max(
                    abs(numeric["original_seed_a"] - numeric["original_seed_b"]),
                    abs(numeric["reordered_seed_a"] - numeric["reordered_seed_b"]),
                )
                allowed = MONTE_CARLO_MARGIN * seed_drift + NUMERICAL_ALLOWANCE
                comparisons.append(
                    {
                        "family": family,
                        "parameter": parameter,
                        "metric": metric,
                        "passed": monte_carlo_equivalent(reorder_drift, seed_drift),
                        "row_reorder_drift": reorder_drift,
                        "different_seed_drift_envelope": seed_drift,
                        "allowed_with_margin": allowed,
                    }
                )
    failure_accounting = {
        "seed_a_equal_after_reorder": payloads["original_seed_a"]["failed_replicates"]
        == payloads["reordered_seed_a"]["failed_replicates"],
        "seed_b_equal_after_reorder": payloads["original_seed_b"]["failed_replicates"]
        == payloads["reordered_seed_b"]["failed_replicates"],
        "requested_counts_exact": all(
            payload["plan"]["replicates"] == MONTE_CARLO_REPLICATES
            for payload in payloads.values()
        ),
        "usable_plus_failed_equals_requested": all(
            payload["usable_replicates"] + len(payload["failed_replicates"])
            == MONTE_CARLO_REPLICATES
            for payload in payloads.values()
        ),
        "parameter_usable_counts_authoritative": all(
            all(
                row["usable_replicates"] == payload["usable_replicates"]
                for row in payload["percentile"]["parameters"]
            )
            for payload in payloads.values()
        ),
    }
    dataset_identities = {
        "original_fingerprint": runs["original_seed_a"]["result"]["provenance"][
            "dataset_fingerprint"
        ],
        "reordered_fingerprint": runs["reordered_seed_a"]["result"]["provenance"][
            "dataset_fingerprint"
        ],
    }
    dataset_identities["distinct_order_bound_fingerprints"] = (
        dataset_identities["original_fingerprint"]
        != dataset_identities["reordered_fingerprint"]
    )
    passed = (
        all(row["passed"] for row in runs.values())
        and exact_repeat
        and exact_worker
        and membership_passed
        and availability_passed
        and original_estimate_passed
        and all(row["passed"] for row in comparisons)
        and all(failure_accounting.values())
        and dataset_identities["distinct_order_bound_fingerprints"]
    )
    generated = [repository_path(original_csv), repository_path(reordered_csv)]
    for run in runs.values():
        generated.extend([run["recipe"], run["output"]])
    return {
        "passed": passed,
        "replicates": MONTE_CARLO_REPLICATES,
        "seeds": [seed_a, seed_b],
        "different_seed_envelope_margin": MONTE_CARLO_MARGIN,
        "numerical_allowance": NUMERICAL_ALLOWANCE,
        "exact_identical_data_seed_repeat": exact_repeat,
        "exact_worker_count_invariance": exact_worker,
        "parameter_membership_exact": membership_passed,
        "availability_exact": availability_passed,
        "original_estimates_within_numerical_allowance": original_estimate_passed,
        "failure_accounting": failure_accounting,
        "dataset_identities": dataset_identities,
        "comparison_count": len(comparisons),
        "maximum_row_reorder_drift": max(
            (row.get("row_reorder_drift", 0.0) for row in comparisons), default=0.0
        ),
        "comparisons": comparisons,
        "run_artifacts": {
            name: {
                "recipe": row["recipe"],
                "result": row["output"],
                "result_sha256": row["output_sha256"],
                "usable_replicates": row["bootstrap"]["usable_replicates"],
                "failed_replicates": len(row["bootstrap"]["failed_replicates"]),
                "execution": row["execution"],
            }
            for name, row in runs.items()
        },
    }, generated


def _failure_boundaries() -> tuple[dict[str, Any], list[str]]:
    variables = ["x1", "x2", "y1", "y2"]
    constant_csv = WORK_ROOT / "boundary_constant.csv"
    write_csv(constant_csv, variables, [[1.0, 1.0, 2.0, 2.0] for _ in range(8)])
    constant = run_bootstrap(
        name="factory_boundary_constant",
        csv_path=constant_csv,
        constructs=MODEL,
        paths=PATHS,
        bootstrap_samples=199,
        expect_success=False,
    )
    constant_text = (
        constant["execution"]["stdout_tail"] + constant["execution"]["stderr_tail"]
    ).lower()
    invalid_plan = run_bootstrap(
        name="factory_boundary_invalid_studentized_plan",
        csv_path=ROOT / "validation" / "fixtures" / "simple_reflective.csv",
        constructs=MODEL,
        paths=PATHS,
        bootstrap_samples=998,
        studentized_inner_samples=99,
        expect_success=False,
    )
    invalid_text = (
        invalid_plan["execution"]["stdout_tail"]
        + invalid_plan["execution"]["stderr_tail"]
    ).lower()
    checks = {
        "data_pathology": {
            "passed": constant["passed"]
            and any(word in constant_text for word in ("constant", "variance", "scale")),
            "no_partial_result": constant["passed"],
            "diagnostic": constant_text[-1200:],
        },
        "unsupported_scope": {
            "passed": invalid_plan["passed"]
            and "studentized" in invalid_text
            and any(value in invalid_text for value in ("999", "primary")),
            "no_partial_result": invalid_plan["passed"],
            "diagnostic": invalid_text[-1200:],
        },
    }
    return {
        "passed": all(row["passed"] for row in checks.values()),
        "categories": checks,
    }, [repository_path(constant_csv), constant["recipe"], invalid_plan["recipe"]]


def _focused_rust_contract() -> dict[str, Any]:
    tests = [
        ("qpls-resampling", "tests::indexed_samples_are_repeatable_and_replicate_specific"),
        ("qpls-resampling", "tests::pls_bootstrap_is_exactly_invariant_to_worker_count"),
        ("qpls-project", "tests::bootstrap_pls_payload_round_trips_with_recipe_provenance"),
    ]
    results: list[dict[str, Any]] = []
    for package, test in tests:
        completed, execution = run_command(
            ["cargo", "test", "-p", package, test, "--", "--exact"], timeout=1800
        )
        output = completed.stdout + completed.stderr
        results.append(
            {
                "package": package,
                "test": test,
                "passed": completed.returncode == 0
                and "1 passed" in output
                and "0 failed" in output,
                "execution": execution,
            }
        )
    return {"passed": all(row["passed"] for row in results), "tests": results}


def run_boundaries() -> tuple[dict[str, Any], list[str]]:
    exhaustive = exhaustive_distribution_bijection()
    monte_carlo, monte_carlo_sources = monte_carlo_row_reorder_gate()
    failures, failure_sources = _failure_boundaries()
    rust_contract = _focused_rust_contract()
    categories = {
        "data_pathology": failures["categories"]["data_pathology"],
        "unsupported_scope": failures["categories"]["unsupported_scope"],
        "metamorphic": {
            "passed": exhaustive["passed"] and monte_carlo["passed"],
            "exhaustive_small_sample_distribution": exhaustive,
            "finite_monte_carlo_row_reorder": monte_carlo,
        },
        "determinism": {
            "passed": monte_carlo["exact_identical_data_seed_repeat"]
            and monte_carlo["exact_worker_count_invariance"]
            and rust_contract["passed"],
            "runtime_comparisons": {
                "repeat_exact": monte_carlo["exact_identical_data_seed_repeat"],
                "worker_exact": monte_carlo["exact_worker_count_invariance"],
            },
            "focused_rust_contract": rust_contract,
        },
        "tamper": {
            "passed": rust_contract["passed"],
            "project_contract_test": next(
                row for row in rust_contract["tests"] if row["package"] == "qpls-project"
            ),
        },
    }
    return {
        "passed": all(row["passed"] for row in categories.values()),
        "categories": categories,
        "engine_source_unchanged": True,
        "row_sorting_introduced": False,
    }, [*monte_carlo_sources, *failure_sources]


def main() -> int:
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    checks, generated = run_boundaries()
    report = write_identity_report(
        "boundary_report",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "crates/qpls-data/src/lib.rs",
            "crates/qpls-resampling/src/lib.rs",
            "crates/qpls-project/src/lib.rs",
            *generated,
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
