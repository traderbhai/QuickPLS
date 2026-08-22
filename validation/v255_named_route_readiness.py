#!/usr/bin/env python3
"""Fail-closed source readiness check for the frozen QuickPLS 2.55 named cases.

This check does not launch QuickPLS or execute a scientific engine.  It proves
that every frozen publication case already has one concrete trusted-driver
route before the first diagnostic pass is allowed to start.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import zipfile
from typing import Any

from v255_release_waiver import (
    DPI_WAIVER_CASE_ID,
    DPI_WAIVER_MANIFEST_DECLARATION,
    exact_approved_waiver_contract,
)


SUITE_ID = "quickpls_v255_named_route_readiness_v1"
TARGET_RELEASE = "2.55.0"
BASE_FIXED = {
    "cross_method:observability:setup screenshot",
    "cross_method:observability:completed Results screenshot",
    "cross_method:accessibility:1024x700",
    "cross_method:observability:machine-readable observation",
    "cross_method:observability:zero unexplained skip",
    "cross_method:observability:running or progress screenshot",
    "cross_method:persistence:save and fresh reopen",
    "cross_method:packaged:offline request observation",
}
TRUSTED_GENERIC_SUITE = (1, "quickpls_v255_named_case_driver_v1")
TRUSTED_CROSS_SUITE = (1, "quickpls_v255_cross_method_candidate_wrapper_v1")
ALLOWED_FIXTURES = {
    "single_mediation", "parallel_mediation", "serial_mediation",
    "simultaneous_two_way", "three_way", "moderated_mediation_first",
    "moderated_mediation_second", "binary_moderation",
    "hoc_rr", "hoc_rf", "hoc_fr", "hoc_ff", "cfa", "recursive_sem",
}
EXPLICIT_ACTIONS = {
    "goto_packaged", "set_viewport", "wait_for", "load_fixture", "set_view", "press",
}


def die(message: str) -> None:
    raise ValueError(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        die(message)


def read_json(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, dict), f"JSON root must be an object: {path}")
    return value


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def safe_zip(path: pathlib.Path) -> None:
    with zipfile.ZipFile(path, "r") as archive:
        names = archive.namelist()
        require("manifest.json" in names and "project.json" in names, f"Curated QPLS fixture lacks manifest/project: {path}")
        require(archive.testzip() is None, f"Curated QPLS fixture has a CRC failure: {path}")
        for info in archive.infolist():
            member = info.filename.replace("\\", "/")
            pure = pathlib.PurePosixPath(member)
            require(not pure.is_absolute() and ".." not in pure.parts and not member.startswith("/"), f"Unsafe QPLS member {member}: {path}")
            require((info.flag_bits & 0x1) == 0, f"Encrypted QPLS member {member}: {path}")
            mode = (info.external_attr >> 16) & 0o170000
            require(mode != 0o120000, f"Symlink QPLS member {member}: {path}")


def git_source_state(repo: pathlib.Path, path: pathlib.Path) -> str:
    relative = path.relative_to(repo).as_posix()
    tracked = subprocess.run(
        ["git", "ls-files", "--error-unmatch", "--", relative],
        cwd=repo, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
    )
    if tracked.returncode == 0:
        return "tracked"
    ignored = subprocess.run(
        ["git", "check-ignore", "-q", "--", relative],
        cwd=repo, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
    )
    require(ignored.returncode != 0, f"Required source input is ignored: {relative}")
    return "untracked_not_ignored"


def matrix_case_ids(matrix: dict[str, Any]) -> tuple[set[str], set[str]]:
    cross: set[str] = set()
    for group, contract in matrix.get("cross_method_evidence", {}).items():
        required = contract.get("required")
        require(isinstance(required, list), f"Matrix cross group lacks required cases: {group}")
        cross.update(f"cross_method:{group}:{case}" for case in required)
    specialized_required = matrix.get("specialized_result_evidence", {}).get("required")
    require(isinstance(specialized_required, list), "Matrix specialized-result cases are absent")
    specialized = {f"specialized_result:{case}" for case in specialized_required}
    require(len(cross) == 29 and len(specialized) == 26, "Frozen matrix must contain exactly 29 cross and 26 specialized cases")
    return cross, specialized


def exact_special_contracts() -> dict[str, dict[str, Any]]:
    def fresh(fixture: str, method: str, table: str, method_version: str, primary: str, execution: str,
              topology: tuple[int, int, int, tuple[int, ...], tuple[str, ...]], **counts: Any) -> dict[str, Any]:
        return {"kind": "fresh_sem_result", "fixture": fixture, "method": method, "table_id": table,
                "method_version": method_version, "primary": primary, "execution": execution,
                "topology": topology, "counts": counts}

    mediation = "qpls3.pls.mediation"
    base = "qpls3.pls.algorithm"
    moderation = "qpls3.pls.general_sem_multiple_two_way_moderation_point"
    hoc = "qpls3.pls.general_sem_higher_order_point"
    cb = "qpls3.cbsem.general_sem_ml"
    contracts: dict[str, dict[str, Any]] = {
        "parallel mediation": fresh("parallel_mediation", "pls_algorithm", "general_sem_specific_indirect_effects", "general_sem_effects_v1", mediation, mediation, (4, 0, 5, (), ()), specific_indirect_count=2),
        "serial mediation": fresh("serial_mediation", "pls_algorithm", "general_sem_specific_indirect_effects", "general_sem_effects_v1", mediation, mediation, (4, 0, 4, (), ()), specific_indirect_count=3),
        "single-mediation bootstrap": fresh("single_mediation", "pls_bootstrap", "general_sem_specific_indirect_effects", "general_sem_pls_single_mediation_full_model_case_bootstrap_v1", mediation, "qpls3.pls.general_sem_single_mediation_bootstrap", (3, 0, 3, (), ()), specific_indirect_count=1),
        "multiple-mediation bootstrap": fresh("parallel_mediation", "pls_bootstrap", "general_sem_specific_indirect_effects", "general_sem_pls_full_model_case_bootstrap_v1", mediation, "qpls3.pls.general_sem_multiple_mediation_bootstrap", (4, 0, 5, (), ()), specific_indirect_count=2),
        "simultaneous two-way moderation": fresh("simultaneous_two_way", "pls_algorithm", "general_sem_interaction_effects", "qpls.general-sem-pls.multiple-two-way.point.v1", moderation, moderation, (4, 0, 5, (2, 2), ()), interaction_effect_count=2, conditional_slope_count=6),
        "three-way moderation": fresh("three_way", "pls_bootstrap", "general_sem_three_way_effect", "qpls.general-sem-pls.three-way.full-model-case-bootstrap.v1", "qpls3.pls.general_sem_three_way_moderation_point", "qpls3.pls.general_sem_three_way_moderation_bootstrap", (4, 0, 7, (2, 2, 2, 3), ()), interaction_effect_count=3, three_way_effect_count=1, three_way_conditional_effect_count=3, three_way_simple_slope_count=9),
        "first-stage moderated mediation": fresh("moderated_mediation_first", "pls_bootstrap", "general_sem_conditional_indirect_effects", "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1", moderation, "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap", (4, 0, 5, (2,), ()), interaction_effect_count=1, conditional_indirect_count=3, moderated_mediation_index_count=1),
        "second-stage moderated mediation": fresh("moderated_mediation_second", "pls_bootstrap", "general_sem_conditional_indirect_effects", "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1", moderation, "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap", (4, 0, 5, (2,), ()), interaction_effect_count=1, conditional_indirect_count=3, moderated_mediation_index_count=1),
        "binary moderator probes": fresh("binary_moderation", "pls_algorithm", "general_sem_conditional_slopes", "qpls.general-sem-pls.multiple-two-way.point.v1", moderation, moderation, (3, 0, 3, (2,), ()), interaction_effect_count=1, conditional_slope_count=2),
        "RF HOC": fresh("hoc_rf", "pls_algorithm", "general_sem_higher_order_targets", "general_sem_pls_higher_order_point_v1", hoc, hoc, (3, 0, 1, (), ("reflective_formative",)), higher_order_stage_count=2),
        "FR HOC": fresh("hoc_fr", "pls_algorithm", "general_sem_higher_order_targets", "general_sem_pls_higher_order_point_v1", hoc, hoc, (3, 0, 1, (), ("formative_reflective",)), higher_order_stage_count=2),
        "FF HOC": fresh("hoc_ff", "pls_algorithm", "general_sem_higher_order_targets", "general_sem_pls_higher_order_point_v1", hoc, hoc, (3, 0, 1, (), ("formative_formative",)), higher_order_stage_count=2),
        "HOC bootstrap": fresh("hoc_rr", "pls_bootstrap", "general_sem_higher_order_targets", "general_sem_pls_higher_order_full_model_case_bootstrap_v1", hoc, "qpls3.pls.general_sem_higher_order_full_model_case_bootstrap", (3, 0, 1, (), ("reflective_reflective",)), higher_order_stage_count=2),
        "Advanced Parameter Table revision": fresh("cfa", "cbsem", "cbsem_general_sem_parameters", "cbsem_general_sem_ml_v1", cb, cb, (3, 3, 0, (), ())),
        "CFA point": fresh("cfa", "cbsem", "cbsem_general_sem_parameters", "cbsem_general_sem_ml_v1", cb, cb, (3, 3, 0, (), ())),
        "recursive SEM point": fresh("recursive_sem", "cbsem", "cbsem_general_sem_parameters", "cbsem_general_sem_ml_v1", cb, cb, (3, 3, 2, (), ())),
        "recursive SEM case bootstrap": fresh("recursive_sem", "cbsem", "cbsem_recursive_sem_bootstrap_inference", "cbsem_general_sem_ml_v1", cb, "qpls3.cbsem.bootstrap.recursive_sem", (3, 3, 2, (), ())),
    }
    archive = {
        "single mediation": ("pls_pm", "specific_indirect_effects", 1),
        "continuous moderator probes": ("pls_pm", "moderation_simple_slopes", 1),
        "RR HOC": ("smartpls.higher_order_models", "general_sem_higher_order_targets", 1),
        "regression OLS": ("regression", "ols_coefficients", 4),
        "regression logistic": ("regression", "logistic_coefficients", 4),
        "regression case bootstrap": ("regression", "regression_bootstrap_summary", 4),
        "PROCESS mediation": ("regression", "process_reference_effects", 6),
        "PROCESS moderation": ("regression", "process_simple_slopes", 11),
    }
    contracts.update({name: {"kind": "archive_result", "method": method, "table_id": table, "minimum_count": count}
                      for name, (method, table, count) in archive.items()})
    contracts["CFA case bootstrap"] = {
        "kind": "fresh_cfa_bootstrap_result", "fixture": "cfa", "method": "cbsem",
        "table_id": "exact_case_bootstrap_parameter_intervals",
        "method_version": "cbsem_ml_exact_parameter_table_v3",
        "primary": "qpls3.cbsem.bootstrap", "execution": "qpls3.cbsem.bootstrap",
        "topology": (3, 3, 0, (), ()),
    }
    require(len(contracts) == 26, "Internal specialized route contract must contain 26 cases")
    return contracts


def route_tokens(route: dict[str, Any], case_id: str) -> None:
    for field in ("header_contains", "row_contains"):
        value = route.get(field)
        require(isinstance(value, list) and value and all(isinstance(item, str) and item for item in value), f"{case_id} requires observed {field} tokens")
    if route["kind"] != "fresh_cfa_bootstrap_result":
        value = route.get("navigation_contains")
        require(isinstance(value, list) and value and all(isinstance(item, str) and item for item in value), f"{case_id} requires observed navigation tokens")


def verify_special_route(repo: pathlib.Path, entry: dict[str, Any], contract: dict[str, Any], source_states: dict[str, str]) -> None:
    case_id = entry["id"]
    route = entry.get("route")
    require(isinstance(route, dict) and route.get("kind") == contract["kind"], f"{case_id} route kind differs from its frozen contract")
    route_tokens(route, case_id)
    if route["kind"] == "archive_result":
        archive = (repo / route.get("archive", "")).resolve()
        require(archive.is_relative_to(repo) and archive.is_file(), f"{case_id} curated archive is missing")
        require(archive.parent == repo / "validation" / "fixtures" / "v255" / "archives", f"{case_id} archive is not a curated v255 source fixture")
        expected_hash = route.get("archive_sha256")
        require(isinstance(expected_hash, str) and len(expected_hash) == 64 and sha256_file(archive) == expected_hash, f"{case_id} archive SHA-256 is absent or wrong")
        safe_zip(archive)
        source_states[archive.relative_to(repo).as_posix()] = git_source_state(repo, archive)
        identity = route.get("archive_identity")
        require(isinstance(identity, dict) and identity.get("result_id") == str(route.get("result_id", "")).removeprefix("canonical:"), f"{case_id} result identity is not exact")
        require(identity.get("method") == contract["method"] and identity.get("table_id") == contract["table_id"], f"{case_id} archive method/table identity drifted")
        count = identity.get("table_row_count", identity.get("table_backing_count"))
        require(isinstance(count, int) and count >= contract["minimum_count"], f"{case_id} archive table inventory is insufficient")
        require(identity.get("status") == "completed" and isinstance(identity.get("method_version"), str), f"{case_id} archive is not an exact completed method result")
        return
    require(route.get("fixture") == contract["fixture"] and route.get("fixture") in ALLOWED_FIXTURES, f"{case_id} fixture differs from its frozen route")
    require(route.get("method") == contract["method"] and route.get("table_id") == contract["table_id"], f"{case_id} method/table differs from its frozen route")
    result = route.get("result")
    require(isinstance(result, dict), f"{case_id} exact result identity is absent")
    require(result.get("method_version") == contract["method_version"] and result.get("primary_cell_id") == contract["primary"] and result.get("execution_cell_id") == contract["execution"], f"{case_id} method/cell identity drifted")
    cells = result.get("capability_cell_ids")
    require(isinstance(cells, list) and len(cells) == len(set(cells)) and contract["primary"] in cells and contract["execution"] in cells, f"{case_id} capability-cell set is incomplete")
    model = route.get("model")
    expected_topology = contract["topology"]
    observed_topology = (
        model.get("ordinary_construct_count") if isinstance(model, dict) else None,
        model.get("common_factor_count") if isinstance(model, dict) else None,
        model.get("structural_relation_count") if isinstance(model, dict) else None,
        tuple(model.get("interaction_orders", [])) if isinstance(model, dict) else (),
        tuple(model.get("higher_order_measurement_types", [])) if isinstance(model, dict) else (),
    )
    require(observed_topology == expected_topology, f"{case_id} exact model topology drifted: {observed_topology!r}")
    if route["kind"] == "fresh_sem_result":
        receipt = route.get("fixture_receipt")
        require(isinstance(receipt, dict) and all(isinstance(receipt.get(key), int) for key in ("constructs", "derived_terms", "paths")), f"{case_id} fixture receipt contract is absent")
        counts = route.get("result_counts", {})
        for key, expected in contract.get("counts", {}).items():
            require(counts.get(key) == expected, f"{case_id} result count {key} must equal {expected}")
        require(isinstance(route.get("route_contains"), list) and route["route_contains"], f"{case_id} lacks observed Calculate routing text")
    if case_id.endswith("Advanced Parameter Table revision"):
        require(route.get("advanced_parameter_revision") is True, "Advanced Parameter Table route must use the visible edit/save/reopen workflow")
    if case_id.endswith("binary moderator probes"):
        probes = route.get("result_counts", {}).get("conditional_probe_contracts")
        require(probes == [{"moderator_id": "construct:b", "kind": "explicit", "values": [0, 1]}], "Binary moderation must assert the exact 0/1 probe contract")
    if "moderated mediation" in case_id:
        expected_stage = "first_stage" if "first-stage" in case_id else "second_stage"
        require(route.get("moderated_stage") == expected_stage, f"{case_id} must bind the exact researcher-selected stage")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=pathlib.Path, default=pathlib.Path(__file__).resolve().parents[1])
    parser.add_argument("--matrix", default="validation/v255_method_evidence_matrix.json")
    parser.add_argument("--index", default="validation/v255_named_evidence_index.json")
    parser.add_argument("--manifest", default="validation/v255_named_case_manifest.json")
    parser.add_argument("--cross-manifest", default="validation/v255_cross_method_case_manifest.json")
    parser.add_argument("--report", type=pathlib.Path)
    parser.add_argument("--require-git-tracked", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = args.repo_root.resolve()
    paths = {name: (repo / getattr(args, name.replace("-", "_"))).resolve() for name in ("matrix", "index", "manifest", "cross_manifest")}
    for name, path in paths.items():
        require(path.is_relative_to(repo) and path.is_file(), f"Required {name} file is absent: {path}")
    matrix, index, manifest, cross_manifest = (read_json(paths[name]) for name in ("matrix", "index", "manifest", "cross_manifest"))
    cross_ids, specialized_ids = matrix_case_ids(matrix)
    frozen_ids = cross_ids | specialized_ids
    index_ids = [entry.get("id") for entry in index.get("entries", [])]
    require(len(index_ids) == 55 and set(index_ids) == frozen_ids and len(set(index_ids)) == 55, "Named index is not the exact frozen 55-case set")
    trusted = {(row.get("schema_version"), row.get("suite_id")) for row in index.get("collector_contract", {}).get("trusted_driver_suites", [])}
    require(TRUSTED_GENERIC_SUITE in trusted and TRUSTED_CROSS_SUITE in trusted, "Named index does not trust both exact schema-1 candidate suites")
    require(exact_approved_waiver_contract(index.get("collector_contract", {}).get("approved_release_waiver")), "Named index lacks the exact product-owner DPI waiver authority")
    require(manifest.get("schema_version") == 1 and manifest.get("suite_id") == "quickpls_v255_named_case_manifest_v1" and manifest.get("target_release") == TARGET_RELEASE, "Named manifest identity is invalid")
    require(manifest.get("status") == "ready" and manifest.get("coverage_status") == "complete" and manifest.get("pending_cases") == [], "Named manifest is not zero-pending and executable")
    fixed = manifest.get("supplied_by_fixed_drivers")
    cases = manifest.get("cases")
    require(isinstance(fixed, list) and isinstance(cases, list), "Named manifest partition arrays are absent")
    curated_ids = [entry.get("id") for entry in cases]
    require(len(fixed) == 25 and len(cases) == 30 and len(set(fixed + curated_ids)) == 55 and set(fixed + curated_ids) == frozen_ids, "Named manifest must be the exact 25/30/0 duplicate-free partition")
    coverage = manifest.get("coverage")
    require(coverage == {"frozen_case_count": 55, "fixed_driver_case_count": 25, "curated_case_count": 30, "pending_case_count": 0}, "Named manifest coverage receipt is wrong")
    cross_driver_ids = {entry.get("id") for entry in cross_manifest.get("cases", [])}
    require(cross_manifest.get("schema_version") == 1 and cross_manifest.get("suite_id") == "quickpls_v255_cross_method_case_manifest_v1" and len(cross_driver_ids) == 17, "Cross-method wrapper manifest identity/count is invalid")
    require(set(fixed) == BASE_FIXED | cross_driver_ids and cross_driver_ids <= cross_ids, "Fixed-driver partition is not the exact base-eight plus cross-wrapper seventeen")
    for entry in cross_manifest.get("cases", []):
        if entry.get("id") == DPI_WAIVER_CASE_ID:
            require({"case_id": DPI_WAIVER_CASE_ID, "status": "waived", **entry.get("approved_waiver", {})} == DPI_WAIVER_MANIFEST_DECLARATION, "Cross-method manifest lacks the exact product-owner DPI waiver authority")
        else:
            require(entry.get("approved_waiver") is None, "Only the exact DPI case may declare an approved waiver")
    generic_cross = [entry for entry in cases if str(entry.get("id", "")).startswith("cross_method:")]
    require(len(generic_cross) == 4, "Generic named driver must own exactly four cross-method UI cases")
    for entry in generic_cross:
        require(entry.get("candidate") in {"portable", "installed"} and isinstance(entry.get("steps"), list) and entry["steps"], f"{entry.get('id')} lacks concrete UI steps")
        require(all(step.get("action") in EXPLICIT_ACTIONS for step in entry["steps"]), f"{entry.get('id')} uses an unsupported explicit action")
        assertion = entry.get("assertion")
        require(isinstance(assertion, dict) and assertion.get("id") == f"{entry.get('operation')}:{entry.get('id')}" and isinstance(assertion.get("query"), dict) and assertion.get("expected") is not None, f"{entry.get('id')} lacks an exact observable assertion")
    special_entries = {entry["id"].split(":", 1)[1]: entry for entry in cases if str(entry.get("id", "")).startswith("specialized_result:")}
    contracts = exact_special_contracts()
    require(set(special_entries) == set(contracts), "Generic driver does not own the exact 26 specialized cases")
    source_states: dict[str, str] = {}
    for name, contract in contracts.items():
        verify_special_route(repo, special_entries[name], contract, source_states)
    required_sources = [
        repo / "validation" / "v255_named_case_driver.mjs",
        repo / "validation" / "v255_named_archive_identity.py",
        repo / "validation" / "windows_native_owned_file_dialog.py",
        repo / "validation" / "run_v255_installed_portable_smoke.ps1",
        repo / "src" / "data" / "v255NamedSemEvidenceFixtures.ts",
    ]
    for source in required_sources:
        require(source.is_file(), f"Required named-route source is missing: {source}")
        source_states[source.relative_to(repo).as_posix()] = git_source_state(repo, source)
    driver_text = required_sources[0].read_text(encoding="utf-8")
    for token in ("archive_result", "fresh_sem_result", "fresh_cfa_bootstrap_result", "exercise_advanced_parameter_revision", "save_and_reopen_case_revision", "expected_sha256", "named_evidence_observations", "candidate-pid", "candidate-path", "--owner-pid", "--owner-executable", "candidate_process"):
        require(token in driver_text, f"Named driver lacks required fail-closed implementation token: {token}")
    wrapper_text = required_sources[3].read_text(encoding="utf-8")
    for token in ("--candidate-pid", "--candidate-path", "candidate_process.pid", "candidate_process.executable_sha256", "candidate_pid_bound", "candidate_executable_bound", "WaiveActualWindows200PercentScaling"):
        require(token in wrapper_text, f"Packaged wrapper lacks required named-driver process binding: {token}")
    if args.require_git_tracked:
        untracked = sorted(path for path, state in source_states.items() if state != "tracked")
        require(not untracked, f"Named route source inputs are not Git-tracked: {untracked}")
    report = {
        "schema_version": 1,
        "suite_id": SUITE_ID,
        "target_release": TARGET_RELEASE,
        "status": "passed",
        "passed": True,
        "partition": {"frozen": 55, "cross": 29, "specialized": 26, "fixed": 25, "curated": 30, "pending": 0},
        "route_kinds": {kind: sum(1 for entry in cases if entry.get("route", {}).get("kind") == kind) for kind in ("archive_result", "fresh_sem_result", "fresh_cfa_bootstrap_result")},
        "trusted_suites": [{"schema_version": schema, "suite_id": suite} for schema, suite in (TRUSTED_GENERIC_SUITE, TRUSTED_CROSS_SUITE)],
        "source_states": source_states,
        "sources": {name: {"path": path.relative_to(repo).as_posix(), "sha256": sha256_file(path)} for name, path in paths.items()},
    }
    encoded = json.dumps(report, separators=(",", ":"), sort_keys=True)
    if args.report:
        output = args.report.resolve()
        require(output.is_relative_to(repo) and not output.exists(), "Readiness report must be a new repository-child path")
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(encoded)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:  # fail closed with one parseable diagnostic
        print(json.dumps({"schema_version": 1, "suite_id": SUITE_ID, "target_release": TARGET_RELEASE,
                          "status": "failed", "passed": False, "error": str(error)}, separators=(",", ":"), sort_keys=True))
        raise SystemExit(1)
