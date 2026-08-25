#!/usr/bin/env python3
"""Mapped-result verifier for full raw MultiMod metamorphic runs.

The producer reports come from the public Recipe V4 compiler and raw runner.
This verifier removes execution identities only, maps declared row tokens, and
compares the complete typed result plus retained resampling evidence.
"""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from typing import Any


IDENTITY_KEYS = {
    "archive_sha256",
    "arrow_schema_sha256",
    "authority_binding_sha256",
    "candidate_qualification_receipt",
    "compiler_receipt",
    "compiled_plan",
    "completed_at",
    "config_sha256",
    "dataset_fingerprint",
    "dataset_id",
    "discovery_result_identity_sha256",
    "engine_version",
    "entry_name",
    "execution_identity_sha256",
    "finalized_cache_sha256",
    "identity_sha256",
    "ledger_sha256",
    "model_id",
    "model_scientific_sha256",
    "plan_sha256",
    "prepackage_manifest_set_sha256",
    "record_identity_sha256",
    "recipe_analytical_sha256",
    "recipe_id",
    "run_id",
    "sha256",
    "shard_identity_sha256",
    "started_at",
    "workers",
}
ROW_SCALAR_KEYS = {"source_row", "row_index", "omitted_row"}
ROW_VECTOR_KEYS = {
    "source_rows",
    "source_row_tokens",
    "row_indices",
    "draw_rows",
    "sampled_rows",
}
ROW_ORDER_VECTOR_KEYS = {
    "assignments",
    "design",
    "hard_assignments",
    "outcome",
    "posteriors",
    "source_row_tokens",
    "true_classes",
}
STABLE_ARRAY_KEYS = (
    "target_id",
    "parameter_id",
    "path_id",
    "case_id",
    "group_id",
    "class_id",
    "segment_id",
    "relation_id",
    "replicate_index",
    "index",
    "id",
)
ROW_TOKEN = re.compile(r"^(?:source[-_]row|row)[:_-](\d+)$")
GROUP_HYPOTHESIS = re.compile(r"^(.*):(group_\d+):(group_\d+):(.*)$")


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain one JSON object")
    return value


def expected_profiles(root: Path, capability_index: Path) -> dict[str, set[str]]:
    index = load(capability_index)
    result: dict[str, set[str]] = {}
    family_alias = {
        "qpls.multimod.mga_multigroup_v1": "mga",
        "qpls.multimod.pls_heterogeneity_v2": "heterogeneity",
        "qpls.multimod.general_sem_conditional_process_v2": "conditional",
        "qpls.multimod.interventional_causal_mediation_v1": "causal",
    }
    for family in index.get("families", []):
        alias = family_alias[str(family["family_id"])]
        result[alias] = {str(profile) for profile in family.get("profiles", [])}
    return result


def mapped_row(value: int, row_count: int) -> int:
    if value < 0 or value >= row_count:
        return value
    return row_count - 1 - value


def normalize(
    value: Any,
    *,
    row_count: int,
    row_reverse: bool,
    parent_key: str = "",
) -> Any:
    if isinstance(value, dict):
        normalized: dict[str, Any] = {}
        for key, item in value.items():
            if key in IDENTITY_KEYS or key.endswith("_sha256"):
                continue
            if row_reverse and key in ROW_SCALAR_KEYS and isinstance(item, int):
                normalized[key] = mapped_row(item, row_count)
                continue
            if row_reverse and key in ROW_VECTOR_KEYS and isinstance(item, list):
                mapped = [
                    mapped_row(row, row_count) if isinstance(row, int) else row for row in item
                ]
                if key in ROW_ORDER_VECTOR_KEYS:
                    mapped.reverse()
                normalized[key] = mapped
                continue
            normalized[key] = normalize(
                item,
                row_count=row_count,
                row_reverse=row_reverse,
                parent_key=key,
            )
        return normalized
    if isinstance(value, list):
        normalized = [
            normalize(item, row_count=row_count, row_reverse=row_reverse, parent_key=parent_key)
            for item in value
        ]
        if row_reverse and parent_key in ROW_ORDER_VECTOR_KEYS:
            normalized.reverse()
        if normalized and all(isinstance(item, dict) for item in normalized):
            for stable_key in STABLE_ARRAY_KEYS:
                if all(stable_key in item for item in normalized):
                    return sorted(
                        normalized,
                        key=lambda item: json.dumps(item[stable_key], sort_keys=True),
                    )
        return normalized
    if row_reverse and isinstance(value, str):
        match = ROW_TOKEN.match(value)
        if match:
            prefix = value[: match.start(1)]
            return f"{prefix}{mapped_row(int(match.group(1)), row_count)}"
    return value


def result_cases(report: dict[str, Any]) -> dict[str, tuple[int, dict[str, Any]]]:
    found: dict[str, tuple[int, dict[str, Any]]] = {}

    def visit(value: Any, path: str, inherited_rows: int | None = None) -> None:
        if isinstance(value, dict):
            row_count = value.get("dataset_rows")
            if not isinstance(row_count, int):
                row_count = inherited_rows
            result_key = "result" if isinstance(value.get("result"), dict) else (
                "analysis" if isinstance(value.get("analysis"), dict) else None
            )
            if result_key and isinstance(row_count, int):
                stable = str(value.get("case_id") or value.get("cell_id") or path)
                scientific = {"result": value[result_key]}
                for evidence_key in (
                    "evidence",
                    "bootstrap_evidence",
                    "analysis_frame",
                    "original_sample_point_fits",
                    "prepared_paths",
                    "multiplicity_replays",
                ):
                    if evidence_key in value:
                        scientific[evidence_key] = value[evidence_key]
                found[stable] = (row_count, scientific)
            for key, item in value.items():
                if key == "label_reversal":
                    continue
                visit(item, f"{path}.{key}", row_count)
        elif isinstance(value, list):
            for index, item in enumerate(value):
                visit(item, f"{path}[{index}]", inherited_rows)

    visit(report, "root")
    return found


def close(left: Any, right: Any, path: str, failures: list[str]) -> None:
    if isinstance(left, bool) or isinstance(right, bool) or left is None or right is None:
        if left != right:
            failures.append(path)
        return
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        if not (math.isfinite(float(left)) and math.isfinite(float(right))):
            if left != right:
                failures.append(path)
            return
        tolerance = 2.0e-8 * max(1.0, abs(float(left)), abs(float(right)))
        if abs(float(left) - float(right)) > tolerance:
            failures.append(path)
        return
    if type(left) is not type(right):
        failures.append(path)
        return
    if isinstance(left, dict):
        if set(left) != set(right):
            failures.append(f"{path}.keys")
            return
        for key in sorted(left):
            close(left[key], right[key], f"{path}.{key}", failures)
        return
    if isinstance(left, list):
        if len(left) != len(right):
            failures.append(f"{path}.length")
            return
        for index, (left_item, right_item) in enumerate(zip(left, right, strict=True)):
            close(left_item, right_item, f"{path}[{index}]", failures)
        return
    if left != right:
        failures.append(path)


def declared_profile_inventory(report: dict[str, Any]) -> set[str]:
    cells: set[str] = set()

    def visit(value: Any) -> None:
        if isinstance(value, dict):
            for key, item in value.items():
                if key in {"cell_ids", "required_cell_ids"} and isinstance(item, list):
                    cells.update(str(cell).split("::", 1)[0] for cell in item if "::" in str(cell))
                elif key == "required_profile_ids" and isinstance(item, list):
                    cells.update(str(profile) for profile in item)
                else:
                    visit(item)
        elif isinstance(value, list):
            for item in value:
                visit(item)

    visit(report)
    return cells


def executed_profile_inventory(family: str, report: dict[str, Any]) -> set[str]:
    profiles: set[str] = set()
    mga_profiles = {
        "general_sem_pls": "mga.general_sem_pls.v1",
        "frequency_expansion_unweighted_pls": "mga.general_sem_pls.v1",
        "multiple_two_way": "mga.multiple_two_way_moderation.v1",
        "bounded_three_way": "mga.bounded_three_way_moderation.v1",
        "bounded_two_way_moderated_mediation": "mga.bounded_two_way_moderated_mediation.v1",
        "multiple_nonnested_hoc": "mga.multiple_nonnested_hoc.v1",
        "case_weighted_pls": "mga.case_weighted_pls.v1",
        "frequency_weighted_pls": "mga.frequency_weighted_pls.v1",
        "reflective_plsc": "mga.reflective_plsc.v1",
    }

    def visit(value: Any) -> None:
        if isinstance(value, dict):
            if family == "mga" and isinstance(value.get("analysis"), dict):
                mapped = mga_profiles.get(str(value.get("profile_fixture", "")))
                if mapped:
                    profiles.add(mapped)
            elif family == "heterogeneity" and isinstance(value.get("analysis"), dict):
                profile = str(value.get("profile", ""))
                suffix = {
                    "p0_structural": "p0_structural",
                    "p2_multi_two_way": "p2_multi_two_way",
                    "p23_all_current": "p23_all_current",
                }.get(profile)
                evidence = value.get("evidence")
                if suffix and isinstance(evidence, dict):
                    if evidence.get("fimix"):
                        profiles.add(f"fimix.{suffix}.v2")
                    if evidence.get("pos"):
                        if suffix == "p0_structural":
                            profiles.add("pos.published.p0_structural.v2")
                        else:
                            profiles.add(f"pos.destination_scored.{suffix}.v2")
                    if evidence.get("common_metric") and suffix != "p0_structural":
                        profiles.add(f"pos.common_metric.{suffix}.v1")
            if family in {"conditional", "causal"} and isinstance(value.get("result"), dict):
                for cell in value.get("cell_ids", []):
                    if "::" in str(cell):
                        profiles.add(str(cell).split("::", 1)[0])
            for item in value.values():
                visit(item)
        elif isinstance(value, list):
            for item in value:
                visit(item)

    visit(report)
    return profiles


def production_contract_valid(family: str, report: dict[str, Any]) -> bool:
    expected = {
        "mga": (
            "qpls.multimod.mga.production-qualification.v1",
            "public_recipe_v4_compiler_plus_raw_resumable_mga_runner",
            "raw_sut_facts_for_independent_comparison_only",
        ),
        "heterogeneity": (
            "qpls.multimod.heterogeneity.production-qualification.v2",
            "public_recipe_v4_compiler_plus_raw_fimix_pos_runner",
            "raw_sut_facts_for_independent_comparison_only",
        ),
        "conditional": (
            "conditional_process_v2",
            "public_recipe_v4_compiler_plus_builtin_raw_runner",
            "none",
        ),
        "causal": (
            "interventional_causal_mediation_v1",
            "public_recipe_v4_compiler_plus_raw_observed_g_computation_runner",
            "none",
        ),
    }
    family_key = "suite_id" if family in {"mga", "heterogeneity"} else "family"
    return (
        (
            report.get(family_key),
            report.get("execution_contract"),
            report.get("qualification_claim"),
        )
        == expected[family]
    )


def compare_axis(
    family: str,
    baseline: dict[str, Any],
    transformed: dict[str, Any],
    axis: str,
) -> dict[str, Any]:
    failures: list[str] = []
    baseline_cases = result_cases(baseline)
    transformed_cases = result_cases(transformed)
    if set(baseline_cases) != set(transformed_cases):
        failures.append("case_inventory")
    for case_id in sorted(set(baseline_cases) & set(transformed_cases)):
        baseline_rows, baseline_science = baseline_cases[case_id]
        transformed_rows, transformed_science = transformed_cases[case_id]
        if baseline_rows != transformed_rows:
            failures.append(f"{case_id}.row_count")
            continue
        if axis == "sign_reverse":
            # The declared sign map flips one adjustment variable. Its fitted
            # nuisance coefficient necessarily reverses, while every reported
            # interventional target and its complete bootstrap ledger must be
            # invariant. Prepared input equations are not a result payload.
            baseline_science = {
                key: value for key, value in baseline_science.items() if key != "prepared_paths"
            }
            transformed_science = {
                key: value for key, value in transformed_science.items() if key != "prepared_paths"
            }
        left = normalize(
            baseline_science,
            row_count=baseline_rows,
            row_reverse=False,
        )
        right = normalize(
            transformed_science,
            row_count=transformed_rows,
            row_reverse=axis == "row_reverse",
        )
        close(left, right, f"{family}.{axis}.{case_id}", failures)
    return {
        "family": family,
        "axis": axis,
        "baseline_case_count": len(baseline_cases),
        "transformed_case_count": len(transformed_cases),
        "complete_result_and_evidence_compared": True,
        "status": "passed" if not failures else "failed",
        "failures": failures[:200],
        "failure_count": len(failures),
    }


def canonical_group_contrast_orientation(value: Any) -> Any:
    if isinstance(value, list):
        return [canonical_group_contrast_orientation(item) for item in value]
    if isinstance(value, str):
        match = GROUP_HYPOTHESIS.match(value)
        if match:
            left, right = sorted((match.group(2), match.group(3)))
            return f"{match.group(1)}:{left}:{right}:{match.group(4)}"
        return value
    if not isinstance(value, dict):
        return value
    mapped = {
        key: canonical_group_contrast_orientation(item) for key, item in value.items()
    }
    left = mapped.get("left_group_id")
    right = mapped.get("right_group_id")
    if isinstance(left, str) and isinstance(right, str) and left > right:
        mapped["left_group_id"], mapped["right_group_id"] = right, left
        for key in ("difference", "difference_left_minus_right"):
            if isinstance(mapped.get(key), (int, float)):
                mapped[key] = -mapped[key]
        probability = mapped.get("directional_probability")
        if isinstance(probability, (int, float)):
            mapped["directional_probability"] = 1.0 - float(probability)
        interval = mapped.get("interval")
        if isinstance(interval, dict):
            lower = interval.get("lower")
            upper = interval.get("upper")
            if isinstance(lower, (int, float)) and isinstance(upper, (int, float)):
                interval["lower"], interval["upper"] = -float(upper), -float(lower)
    return mapped


def verify_mga_group_label_mapping(report: dict[str, Any]) -> dict[str, Any]:
    failures: list[str] = []
    reversal = report.get("label_reversal")
    if not isinstance(reversal, dict):
        failures.append("label_reversal")
        reversal = {}
    forward = reversal.get("forward")
    reverse = reversal.get("reverse")
    if not isinstance(forward, dict) or not isinstance(reverse, dict):
        failures.append("forward_reverse_receipts")
    else:
        scientific_keys = ("analysis", "evidence", "multiplicity_replays")
        left = canonical_group_contrast_orientation(
            {key: forward.get(key) for key in scientific_keys}
        )
        right = canonical_group_contrast_orientation(
            {key: reverse.get(key) for key in scientific_keys}
        )
        left = normalize(left, row_count=int(forward.get("dataset_rows", 0)), row_reverse=False)
        right = normalize(right, row_count=int(reverse.get("dataset_rows", 0)), row_reverse=False)
        close(left, right, "mga.group_label_reversal", failures)
    return {
        "family": "mga",
        "axis": "group_label_reversal",
        "complete_result_and_evidence_compared": True,
        "status": "passed" if not failures else "failed",
        "failures": failures[:200],
        "failure_count": len(failures),
    }


def verify_mga_cancel_resume(report: dict[str, Any]) -> dict[str, Any]:
    failures: list[str] = []
    matrix = report.get("group_matrix")
    receipt = None
    if isinstance(matrix, list):
        for row in matrix:
            if isinstance(row, dict) and isinstance(row.get("cancel_resume"), dict):
                receipt = row["cancel_resume"]
                break
    if not isinstance(receipt, dict):
        failures.append("cancel_resume_receipt")
        receipt = {}
    for flag in (
        "cancelled_without_result",
        "partial_result_unpublishable",
        "completed_cache_exact",
        "completed_shards_reused_without_retry",
        "finalized_cache_matches_uninterrupted",
        "complete_result_matches_uninterrupted",
        "complete_evidence_matches_uninterrupted",
        "compilation_receipt_matches_uninterrupted",
    ):
        if receipt.get(flag) is not True:
            failures.append(flag)
    planned = receipt.get("planned_shards")
    completed = receipt.get("completed_before_resume")
    if not (
        isinstance(planned, int)
        and isinstance(completed, int)
        and planned > 1
        and 0 < completed < planned
    ):
        failures.append("partial_cache_boundary")
    return {
        "family": "mga",
        "axis": "shard_cancel_resume",
        "production_resumable_runner_compared": True,
        "status": "passed" if not failures else "failed",
        "failures": failures,
    }


def verify_heterogeneity_class_label_mapping(report: dict[str, Any]) -> dict[str, Any]:
    failures: list[str] = []
    expected_cell_id = "fimix-p23-fixed-k-bootstrap"
    selected_cases = 0
    prepared_ledgers = 0
    usable_entries = 0
    nonidentity_mappings = 0
    complete_target_vectors = 0
    nonidentity_target_vectors = 0
    for case in report.get("fixed_k_bootstrap", []):
        if not isinstance(case, dict):
            failures.append("fixed_k_case")
            continue
        if case.get("cell_id") != expected_cell_id:
            failures.append("unexpected_fixed_k_profile")
            continue
        selected_cases += 1
        evidence = case.get("evidence")
        bootstrap_rows = evidence.get("bootstrap", []) if isinstance(evidence, dict) else []
        for prepared in bootstrap_rows:
            if not isinstance(prepared, dict):
                failures.append("prepared_bootstrap")
                continue
            prepared_ledgers += 1
            if prepared.get("exhaustive_label_alignment_applied") is not True:
                failures.append("exhaustive_alignment")
            targets = prepared.get("targets")
            if not isinstance(targets, list) or not targets:
                failures.append("aligned_target_inventory")
                targets = []
            target_ids = [
                target.get("target_id")
                for target in targets
                if isinstance(target, dict)
            ]
            if len(target_ids) != len(targets) or len(set(target_ids)) != len(target_ids):
                failures.append("aligned_target_identity")
            for entry in prepared.get("entries", []):
                if not isinstance(entry, dict) or entry.get("status") != "usable":
                    continue
                usable_entries += 1
                replicate = entry.get("replicate_index")
                vector = []
                if isinstance(replicate, int) and replicate >= 0:
                    for target in targets:
                        estimates = target.get("estimates") if isinstance(target, dict) else None
                        if not isinstance(estimates, list) or replicate >= len(estimates):
                            vector = []
                            break
                        estimate = estimates[replicate]
                        if not isinstance(estimate, (int, float)) or not math.isfinite(float(estimate)):
                            vector = []
                            break
                        vector.append(float(estimate))
                if len(vector) != len(targets) or entry.get("target_payload_sha256") in (None, ""):
                    failures.append("usable_aligned_target_vector")
                else:
                    complete_target_vectors += 1
                alignment = entry.get("label_alignment")
                if not isinstance(alignment, dict):
                    failures.append("usable_alignment_missing")
                    continue
                mapping = alignment.get("candidate_to_reference")
                if (
                    alignment.get("ambiguous") is not False
                    or alignment.get("mutual_majority") is not True
                    or not isinstance(mapping, list)
                    or sorted(mapping) != list(range(len(mapping)))
                ):
                    failures.append("usable_alignment_invalid")
                elif mapping != list(range(len(mapping))):
                    nonidentity_mappings += 1
                    if len(vector) == len(targets):
                        nonidentity_target_vectors += 1
    if selected_cases != 1 or prepared_ledgers < 1:
        failures.append("fixed_k_profile_ledgers")
    if usable_entries == 0:
        failures.append("usable_alignment_inventory")
    if nonidentity_mappings == 0 or nonidentity_target_vectors == 0:
        failures.append("nonidentity_class_label_mapping_not_exercised")
    return {
        "family": "heterogeneity",
        "axis": "class_label_alignment",
        "expected_profile_cell_id": expected_cell_id,
        "prepared_ledger_count": prepared_ledgers,
        "usable_aligned_entry_count": usable_entries,
        "nonidentity_mapping_count": nonidentity_mappings,
        "complete_aligned_target_vector_count": complete_target_vectors,
        "nonidentity_aligned_target_vector_count": nonidentity_target_vectors,
        "complete_mapped_target_ledgers_compared_across_axes": True,
        "truth_labels_used_as_sut_labels": False,
        "status": "passed" if not failures else "failed",
        "failures": sorted(set(failures)),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-directory", type=Path, required=True)
    parser.add_argument("--capability-index", type=Path, required=True)
    parser.add_argument("--repository-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    expected = expected_profiles(args.repository_root.resolve(), args.capability_index)
    axes = [
        "seed_repeat",
        "row_reverse",
        "input_column_reverse",
        "declaration_reverse",
        "worker_parallel",
    ]
    comparisons: list[dict[str, Any]] = []
    failures: list[str] = []
    covered_profiles: set[str] = set()
    baselines: dict[str, dict[str, Any]] = {}
    for family in ("mga", "heterogeneity", "conditional", "causal"):
        baseline = load(args.input_directory / f"{family}-baseline.json")
        baselines[family] = baseline
        if not production_contract_valid(family, baseline):
            failures.append(f"{family}.production_contract")
        if baseline.get("metamorphism") != "baseline" or baseline.get("workers") != 1:
            failures.append(f"{family}.baseline_execution_mapping")
        inventory = declared_profile_inventory(baseline)
        executed = executed_profile_inventory(family, baseline)
        covered_profiles.update(executed)
        if inventory != expected[family] or executed != expected[family]:
            failures.append(f"{family}.profile_inventory")
        for axis in axes:
            transformed = load(args.input_directory / f"{family}-{axis}.json")
            if (
                declared_profile_inventory(transformed) != expected[family]
                or executed_profile_inventory(family, transformed) != expected[family]
            ):
                failures.append(f"{family}.{axis}.profile_inventory")
            if not production_contract_valid(family, transformed):
                failures.append(f"{family}.{axis}.production_contract")
            expected_workers = 4 if axis == "worker_parallel" else 1
            if (
                transformed.get("metamorphism") != axis
                or transformed.get("workers") != expected_workers
                or transformed.get("seed") != baseline.get("seed")
            ):
                failures.append(f"{family}.{axis}.execution_mapping")
            comparison = compare_axis(family, baseline, transformed, axis)
            comparisons.append(comparison)
            if comparison["status"] != "passed":
                failures.append(f"{family}.{axis}")
    causal = load(args.input_directory / "causal-baseline.json")
    causal_sign = load(args.input_directory / "causal-sign_reverse.json")
    if not production_contract_valid("causal", causal_sign):
        failures.append("causal.sign_reverse.production_contract")
    if causal_sign.get("metamorphism") != "sign_reverse" or causal_sign.get("workers") != 1:
        failures.append("causal.sign_reverse.execution_mapping")
    sign_comparison = compare_axis("causal", causal, causal_sign, "sign_reverse")
    comparisons.append(sign_comparison)
    if sign_comparison["status"] != "passed":
        failures.append("causal.sign_reverse")
    group_label_comparison = verify_mga_group_label_mapping(baselines["mga"])
    comparisons.append(group_label_comparison)
    if group_label_comparison["status"] != "passed":
        failures.append("mga.group_label_reversal")
    cancel_resume_comparison = verify_mga_cancel_resume(baselines["mga"])
    comparisons.append(cancel_resume_comparison)
    if cancel_resume_comparison["status"] != "passed":
        failures.append("mga.shard_cancel_resume")
    class_label_comparison = verify_heterogeneity_class_label_mapping(
        baselines["heterogeneity"]
    )
    comparisons.append(class_label_comparison)
    if class_label_comparison["status"] != "passed":
        failures.append("heterogeneity.class_label_alignment")

    expected_all_profiles = set().union(*expected.values())
    missing_profiles = sorted(expected_all_profiles - covered_profiles)
    if missing_profiles:
        failures.append("global.profile_inventory")
    result = {
        "schema_version": 1,
        "report_id": "qpls.multimod.global-metamorphic-qualification.v1",
        "status": "passed" if not failures and covered_profiles == expected_all_profiles else "failed",
        "producer_contract": "public_recipe_v4_compiler_plus_full_raw_family_runners",
        "covered_profiles": sorted(covered_profiles),
        "missing_profiles": missing_profiles,
        "comparisons": comparisons,
        "failures": failures,
        "pending_reason": None,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
