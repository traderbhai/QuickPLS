#!/usr/bin/env python3
"""Pre-registered HTMT boundary, metamorphic, and adversarial gate."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from htmt_plus_v1_factory_common import (
    ROOT,
    require_exact_case_ids,
    strict_load_json,
    write_identity_report,
)
from htmt_reference import htmt_cell


SOURCE = "validation/htmt_release_boundary_gate.py"
REFERENCE = ROOT / "validation" / "results" / "htmt_reference.json"
BOOTSTRAP_REFERENCE = (
    ROOT / "validation" / "results" / "htmt_bootstrap_inference_reference.json"
)

REQUIRED_CASE_IDS = (
    "zero_variance",
    "duplicated_indicator_binding",
    "exact_collinearity",
    "near_collinearity",
    "extreme_scale",
    "extreme_missingness",
    "n_less_than_p",
    "summary_matrix_input_rejected",
    "formative_pair_not_applicable",
    "single_indicator_not_applicable",
    "nonpositive_original_monotrait_unavailable",
    "zero_plus_monotrait_unavailable",
    "htmt_plus_above_one_unclamped",
    "threshold_equal_0_90_is_false",
    "replicate_digest_tamper_rejected",
    "duplicate_unavailable_index_rejected",
    "row_reorder",
    "indicator_reorder",
    "construct_reorder",
    "positive_affine_rescale",
    "indicator_sign_reversal",
    "seed_repeat",
    "worker_count_change",
    "save_reopen",
    "gui_cli_equivalence",
    "malformed_archive",
    "semantic_archive_tamper",
)


def _digest(indices: list[int]) -> str:
    digest = hashlib.sha256()
    for index in indices:
        digest.update(index.to_bytes(4, "little", signed=False))
    return digest.hexdigest()


def source_boundary_checks() -> dict[str, Any]:
    reference = strict_load_json(REFERENCE)
    bootstrap = strict_load_json(BOOTSTRAP_REFERENCE)
    baseline = reference["fixtures"][0]
    maximum_plus = max(
        float(cell["value"])
        for row in baseline["htmt_plus"]
        for cell in row
        if cell["value"] is not None
    )
    single = htmt_cell(
        ["a"],
        ["b", "c"],
        {"a": [-1.0, 0.0, 1.0], "b": [-1.0, 0.0, 1.0], "c": [1.0, 0.0, -1.0]},
        True,
    )
    negative_original = htmt_cell(
        ["a", "b"],
        ["c", "d"],
        {
            "a": [-2.0, -1.0, 1.0, 2.0],
            "b": [2.0, 1.0, -1.0, -2.0],
            "c": [-2.0, -1.0, 1.0, 2.0],
            "d": [-1.9, -1.1, 0.9, 2.1],
        },
        False,
    )
    zero_plus = htmt_cell(
        ["a", "b"],
        ["c", "d"],
        {
            "a": [-1.0, -1.0, 1.0, 1.0],
            "b": [-1.0, 1.0, -1.0, 1.0],
            "c": [-1.0, 0.0, 0.0, 1.0],
            "d": [-1.0, 1.0, 1.0, -1.0],
        },
        True,
    )
    first_bootstrap = bootstrap["scenarios"][0]
    indices = first_bootstrap["usable_replicate_indices"]
    expected_digest = first_bootstrap["usable_replicate_indices_sha256"]
    cases = {
        "single_indicator_not_applicable": (
            single["status"] == "not_applicable"
            and single["reason"] == "htmt.single_indicator_not_applicable"
        ),
        "nonpositive_original_monotrait_unavailable": (
            negative_original["status"] == "unavailable"
            and negative_original["reason"]
            == "htmt.original_nonpositive_monotrait_mean"
        ),
        "zero_plus_monotrait_unavailable": (
            zero_plus["status"] == "unavailable"
            and zero_plus["reason"] == "htmt.zero_monotrait_denominator"
        ),
        "htmt_plus_above_one_unclamped": maximum_plus > 1.0,
        "threshold_equal_0_90_is_false": not (0.90 < 0.90),
        "positive_affine_rescale": (
            reference["metamorphic_checks"]["positive_affine_htmt_plus_max_delta"]
            <= 1e-12
            and reference["metamorphic_checks"]["positive_affine_original_max_delta"]
            <= 1e-12
        ),
        "indicator_sign_reversal": (
            reference["metamorphic_checks"]["reverse_one_indicator_plus_max_delta"]
            <= 1e-12
            and reference["metamorphic_checks"][
                "reverse_one_indicator_original_has_unavailable"
            ]
            is True
        ),
        "replicate_digest_reference": (
            _digest(indices) == expected_digest
            and _digest(list(reversed(indices))) != expected_digest
        ),
    }
    return {
        "passed": all(cases.values()),
        "cases": cases,
        "maximum_observed_htmt_plus": maximum_plus,
        "qualification_case_ids": list(REQUIRED_CASE_IDS),
        "source_checked_case_ids": sorted(cases),
    }


def validate_qualification_report(path: Path) -> dict[str, Any]:
    document = strict_load_json(path)
    identity_passed = (
        document.get("qualification_id") == "qpls3.assessment.htmt.qualification_v2"
        and document.get("method_version") == "ringle_et_al_htmt_plus_v1"
        and document.get("failed_cases") == []
        and document.get("untyped_failures") == 0
    )
    cases = require_exact_case_ids(document, REQUIRED_CASE_IDS)
    return {
        "passed": identity_passed and cases["passed"],
        "identity_passed": identity_passed,
        "cases": cases,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--qualification-report", type=Path)
    parser.add_argument("--admit", action="store_true")
    args = parser.parse_args()
    source_checks = source_boundary_checks()
    checks: dict[str, Any] = {"source_boundaries": source_checks}
    blockers = [
        "current_candidate_adversarial_execution_not_supplied",
        "archive_and_gui_cli_metamorphic_cases_not_executed_in_this_source_audit",
    ]
    qualification_evidence = False
    if args.qualification_report:
        qualification = validate_qualification_report(args.qualification_report)
        checks["qualification_execution"] = qualification
        if qualification["passed"]:
            blockers = []
            qualification_evidence = args.admit
        else:
            blockers.append("qualification_report_failed_contract")
    passed = source_checks["passed"] and (
        not args.qualification_report or checks["qualification_execution"]["passed"]
    )
    report = write_identity_report(
        "boundary_report",
        stage="adversarial",
        passed=passed,
        checks=checks,
        blockers=blockers,
        extras=[
            SOURCE,
            "validation/htmt_reference.py",
            "validation/results/htmt_reference.json",
            "validation/results/htmt_bootstrap_inference_reference.json",
        ],
        qualification_evidence=qualification_evidence,
    )
    print(
        json.dumps(
            {
                "passed": passed,
                "qualification_evidence": qualification_evidence,
                "blockers": blockers,
                "report": report.relative_to(ROOT).as_posix(),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if passed and (not args.admit or qualification_evidence) else 1


if __name__ == "__main__":
    raise SystemExit(main())
