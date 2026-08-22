"""Fail-closed deterministic QuickPLS 2.55 evidence-bundle assembler.

This tool does not collect evidence.  It accepts only already passing named and
frozen crawler outputs, derives the frozen index from parsed receipts, verifies
every staged byte, and writes new proposal files plus a new ZIP.  It never
overwrites an input or output.
"""

from __future__ import annotations

import argparse
import copy
from collections import Counter
import hashlib
import json
import stat
import zipfile
from pathlib import Path, PurePosixPath
from typing import Any

from v255_release_waiver import (
    DPI_WAIVER_MANIFEST_DECLARATION,
    exact_case_waiver_receipt_matches_observation,
    exact_cross_report_waiver_binding,
    exact_population_status,
    exact_release_waiver_matches_observation,
)

ROOT = Path(__file__).resolve().parents[1]
TARGET = "2.55.0"
FROZEN_SUITE = "quickpls_v255_frozen_archive_reopen_crawler_v1"
COLLECTOR_SUITE = "quickpls_v255_named_evidence_collector_v1"
TRUSTED_DRIVER_SUITES = {
    "quickpls_v255_live_calculation_lifecycle_smoke_v1": 1,
    "quickpls_v255_method_evidence_crawler_v2": 2,
    "quickpls_v255_frozen_archive_reopen_crawler_v1": 1,
    "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1": 1,
    "quickpls_v255_named_case_driver_v1": 1,
    "quickpls_v255_cross_method_candidate_wrapper_v1": 1,
}
EXPECTED_CANDIDATE_DRIVER_ROLE_SUITES = {
    "installed": {
        "lifecycle": "quickpls_v255_live_calculation_lifecycle_smoke_v1",
        "method_evidence": "quickpls_v255_method_evidence_crawler_v2",
        "named_evidence_driver_0": "quickpls_v255_named_case_driver_v1",
    },
    "portable": {
        "lifecycle": "quickpls_v255_live_calculation_lifecycle_smoke_v1",
        "method_evidence": "quickpls_v255_method_evidence_crawler_v2",
        "frozen_archive_reopen": "quickpls_v255_frozen_archive_reopen_crawler_v1",
        "posthoc_execute": "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1",
        "posthoc_reopen": "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1",
        "named_evidence_driver_0": "quickpls_v255_named_case_driver_v1",
        "named_evidence_driver_1": "quickpls_v255_cross_method_candidate_wrapper_v1",
    },
}
CROSS_WRAPPER_SUITE = "quickpls_v255_cross_method_candidate_wrapper_v1"
CROSS_RENDERER_SUITE = "quickpls_v255_cross_method_candidate_driver_v1"
CROSS_NATIVE_GUARD_SUITE = "quickpls_v255_windows_unsaved_close_guard_v1"
CROSS_RENDERER_PHASES = (
    "imports",
    "exports",
    "archives",
    "legacy_reopen",
    "autosave_seed",
    "autosave_recover",
    "unsaved_close_seed",
    "dpi_process",
)
CROSS_PHASES = (*CROSS_RENDERER_PHASES[:-1], "unsaved_close_guard", "dpi_process")
FROZEN_OBSERVATION_SOURCES = frozenset(
    {
        "navigation",
        "visible_headings",
        "visible_result_table_ids",
        "table_titles",
        "table_headers",
        "table_rows",
        "chart_titles",
    }
)
FROZEN_TABLE_OBSERVATION_SOURCES = frozenset(
    {"visible_result_table_ids", "table_titles", "table_headers", "table_rows"}
)
FROZEN_OBSERVATION_MATCHERS = frozenset({"exact", "contains"})


def fail(message: str) -> None:
    raise ValueError(message)


def load(path: Path) -> dict[str, Any]:
    if not path.is_file() or path.stat().st_size > 32 * 1024 * 1024:
        fail(f"missing or oversized JSON: {path}")
    value = json.loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(value, dict):
        fail(f"JSON root is not an object: {path}")
    return value


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_digest(value: Any) -> str:
    payload = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def exact_empty_console_errors(payload: object) -> bool:
    return isinstance(payload, dict) and payload.get("console_errors") == []


def exact_schema_version(payload: object, expected: object) -> bool:
    return (
        isinstance(payload, dict)
        and type(expected) is int
        and type(payload.get("schema_version")) is int
        and payload.get("schema_version") == expected
    )


def hash_matches(declared: object, actual: object) -> bool:
    return (
        isinstance(declared, str)
        and isinstance(actual, str)
        and len(declared) == len(actual) == 64
        and all(character in "0123456789abcdefABCDEF" for character in declared)
        and all(character in "0123456789abcdefABCDEF" for character in actual)
        and declared.casefold() == actual.casefold()
    )


def same_declared_path(left: object, right: object) -> bool:
    if not isinstance(left, str) or not left or not isinstance(right, str) or not right:
        return False
    try:
        return Path(left).resolve() == Path(right).resolve()
    except (OSError, ValueError):
        return False


def driver_report_passed(payload: dict[str, Any]) -> bool:
    return (
        payload.get("target_release") == TARGET
        and (
            payload.get("passed") is True
            or (
                "passed" not in payload
                and payload.get("status") in {"passed", "verified"}
            )
        )
        and exact_empty_console_errors(payload)
    )


def nested(value: object, *keys: str) -> object:
    current = value
    for key in keys:
        if not isinstance(current, dict):
            return None
        current = current.get(key)
    return current


def candidate_driver_binding_inventory(
    candidate_report: object,
) -> list[tuple[str, str, str, str]] | None:
    if not isinstance(candidate_report, dict):
        return None
    outcomes = candidate_report.get("outcomes")
    if not isinstance(outcomes, list):
        return None
    inventory: list[tuple[str, str, str, str]] = []
    seen_candidates: set[str] = set()
    for outcome in outcomes:
        if not isinstance(outcome, dict):
            return None
        candidate = outcome.get("name")
        if (
            not isinstance(candidate, str)
            or candidate not in {"installed", "portable"}
            or candidate in seen_candidates
        ):
            return None
        seen_candidates.add(candidate)
        expected_role_suites = EXPECTED_CANDIDATE_DRIVER_ROLE_SUITES[candidate]
        fixed_bindings: dict[str, tuple[object, object]] = {
            "lifecycle": (outcome.get("lifecycle"), outcome.get("lifecycle_sha256")),
            "method_evidence": (
                outcome.get("evidence"),
                outcome.get("evidence_sha256"),
            ),
            "frozen_archive_reopen": (
                nested(outcome, "frozen_archive_collection", "aggregate_receipt"),
                nested(
                    outcome,
                    "frozen_archive_collection",
                    "aggregate_receipt_sha256",
                ),
            ),
            "posthoc_execute": (
                nested(outcome, "posthoc_collection", "execute_receipt"),
                nested(outcome, "posthoc_collection", "execute_receipt_sha256"),
            ),
            "posthoc_reopen": (
                nested(outcome, "posthoc_collection", "reopen_receipt"),
                nested(outcome, "posthoc_collection", "reopen_receipt_sha256"),
            ),
        }
        for role, (path, declared_hash) in fixed_bindings.items():
            if role not in expected_role_suites:
                if path is not None or declared_hash is not None:
                    return None
                continue
            if not (
                isinstance(path, str)
                and bool(path)
                and isinstance(declared_hash, str)
                and len(declared_hash) == 64
                and all(
                    character in "0123456789abcdef" for character in declared_hash
                )
            ):
                return None
            inventory.append(
                (candidate, role, declared_hash, expected_role_suites[role])
            )

        additional = outcome.get("named_evidence_driver_reports")
        expected_additional_count = sum(
            role.startswith("named_evidence_driver_") for role in expected_role_suites
        )
        if not isinstance(additional, list) or len(additional) != expected_additional_count:
            return None
        for index, row in enumerate(additional):
            if not isinstance(row, dict) or set(row) != {"path", "sha256"}:
                return None
            role = f"named_evidence_driver_{index}"
            path = row.get("path")
            declared_hash = row.get("sha256")
            if not (
                role in expected_role_suites
                and isinstance(path, str)
                and bool(path)
                and isinstance(declared_hash, str)
                and len(declared_hash) == 64
                and all(
                    character in "0123456789abcdef" for character in declared_hash
                )
            ):
                return None
            inventory.append(
                (candidate, role, declared_hash, expected_role_suites[role])
            )
    return inventory if seen_candidates == {"installed", "portable"} else None


def exact_candidate_driver_inventory(
    candidate_report: object, trusted_rows: object
) -> bool:
    expected = candidate_driver_binding_inventory(candidate_report)
    if expected is None or not isinstance(trusted_rows, list):
        return False
    observed: list[tuple[str, str, str, str]] = []
    for row in trusted_rows:
        if not isinstance(row, dict):
            return False
        candidate = row.get("candidate")
        role = row.get("role")
        declared_hash = row.get("sha256")
        suite_id = row.get("suite_id")
        if not (
            isinstance(candidate, str)
            and isinstance(role, str)
            and isinstance(declared_hash, str)
            and len(declared_hash) == 64
            and all(character in "0123456789abcdef" for character in declared_hash)
            and isinstance(suite_id, str)
        ):
            return False
        observed.append((candidate, role, declared_hash, suite_id))
    return Counter(expected) == Counter(observed)


def exact_cross_phase_payloads(
    wrapper: dict[str, Any],
    records: object,
    *,
    expected_candidate_sha256: object,
    expected_candidate_path: object,
    expected_product_version: object,
    expected_source_commit: object,
) -> bool:
    if not isinstance(records, list) or len(records) != len(CROSS_PHASES):
        return False
    if any(
        not isinstance(record, dict)
        or set(record)
        != {
            "phase",
            "member",
            "sha256",
            "suite_id",
            "schema_version",
            "renderer_attached",
            "payload",
        }
        for record in records
    ):
        return False
    if any(
        not isinstance(record.get("phase"), str)
        or not isinstance(record.get("member"), str)
        or not isinstance(record.get("sha256"), str)
        for record in records
    ):
        return False
    record_by_phase = {
        record.get("phase"): record for record in records if isinstance(record, dict)
    }
    if (
        len(record_by_phase) != len(CROSS_PHASES)
        or set(record_by_phase) != set(CROSS_PHASES)
        or len({record.get("member") for record in records}) != len(CROSS_PHASES)
        or len({record.get("sha256") for record in records}) != len(CROSS_PHASES)
    ):
        return False
    bindings = wrapper.get("phase_reports")
    if not isinstance(bindings, list) or len(bindings) != len(CROSS_PHASES):
        return False
    if any(
        not isinstance(binding, dict)
        or set(binding) != {"phase", "path", "sha256"}
        for binding in bindings
    ):
        return False
    if any(
        not isinstance(binding.get("phase"), str)
        or not isinstance(binding.get("path"), str)
        or not isinstance(binding.get("sha256"), str)
        for binding in bindings
    ):
        return False
    binding_by_phase = {
        binding.get("phase"): binding
        for binding in bindings
        if isinstance(binding, dict)
    }
    if (
        len(binding_by_phase) != len(CROSS_PHASES)
        or set(binding_by_phase) != set(CROSS_PHASES)
        or len({binding.get("path") for binding in bindings}) != len(CROSS_PHASES)
        or len({str(binding.get("sha256")).casefold() for binding in bindings})
        != len(CROSS_PHASES)
    ):
        return False

    wrapper_candidate = wrapper.get("candidate")
    wrapper_candidate = wrapper_candidate if isinstance(wrapper_candidate, dict) else {}
    if not (
        wrapper_candidate.get("role") == "portable"
        and hash_matches(
            wrapper_candidate.get("sha256"), expected_candidate_sha256
        )
        and same_declared_path(
            wrapper_candidate.get("path"), expected_candidate_path
        )
        and wrapper_candidate.get("product_version") == expected_product_version
        and wrapper.get("source_commit") == expected_source_commit
    ):
        return False
    wrapper_sha = wrapper_candidate.get("sha256")
    wrapper_path = wrapper_candidate.get("path")
    renderer_pids: list[int] = []
    for phase in CROSS_PHASES:
        record = record_by_phase[phase]
        phase_payload = record.get("payload")
        expected_suite = (
            CROSS_NATIVE_GUARD_SUITE
            if phase == "unsaved_close_guard"
            else CROSS_RENDERER_SUITE
        )
        if not (
            hash_matches(binding_by_phase[phase].get("sha256"), record.get("sha256"))
            and record.get("suite_id") == expected_suite
            and exact_schema_version(record, 1)
            and record.get("renderer_attached")
            is (phase != "unsaved_close_guard")
            and isinstance(phase_payload, dict)
            and exact_schema_version(phase_payload, 1)
            and phase_payload.get("suite_id") == expected_suite
            and phase_payload.get("passed") is True
            and phase_payload.get("failures") == []
        ):
            return False
        if phase != "unsaved_close_guard":
            phase_candidate = phase_payload.get("candidate")
            phase_candidate = phase_candidate if isinstance(phase_candidate, dict) else {}
            if not (
                phase_payload.get("target_release") == TARGET
                and phase_payload.get("phase") == phase
                and exact_empty_console_errors(phase_payload)
                and isinstance(phase_payload.get("offline"), dict)
                and phase_payload["offline"].get("passed") is True
                and type(phase_candidate.get("pid")) is int
                and phase_candidate.get("pid", 0) > 0
                and hash_matches(phase_candidate.get("sha256"), wrapper_sha)
                and same_declared_path(phase_candidate.get("path"), wrapper_path)
                and phase_payload.get("source_commit") == wrapper.get("source_commit")
            ):
                return False
            renderer_pids.append(phase_candidate["pid"])

    process_safety = wrapper.get("process_safety")
    process_safety = process_safety if isinstance(process_safety, dict) else {}
    terminations = process_safety.get("terminations")
    termination_rows = (
        [row for row in terminations if isinstance(row, dict)]
        if isinstance(terminations, list)
        else []
    )
    root_pids = [row.get("root_pid") for row in termination_rows]
    sentinel_pid = process_safety.get("sentinel_pid")
    if not (
        len(renderer_pids) == len(set(renderer_pids)) == len(CROSS_RENDERER_PHASES)
        and isinstance(terminations, list)
        and len(terminations) == len(termination_rows)
        and len(termination_rows) == len(root_pids) == len(CROSS_RENDERER_PHASES)
        and all(type(pid) is int and pid > 0 for pid in root_pids)
        and len(set(root_pids)) == len(CROSS_RENDERER_PHASES)
        and set(root_pids) == set(renderer_pids)
        and all(
            row.get("exact_tree_terminated") is True
            and row.get("endpoint_closed") is True
            for row in termination_rows
        )
        and process_safety.get("exact_pid_tree_cleanup_only") is True
        and process_safety.get("no_existing_candidate_attached") is True
        and process_safety.get("sentinel_survived_candidate_cleanup") is True
        and type(sentinel_pid) is int
        and sentinel_pid > 0
        and sentinel_pid not in set(renderer_pids)
    ):
        return False

    seed_candidate = record_by_phase["unsaved_close_seed"]["payload"].get("candidate")
    guard_payload = record_by_phase["unsaved_close_guard"]["payload"]
    guard_candidate = guard_payload.get("candidate")
    seed_candidate = seed_candidate if isinstance(seed_candidate, dict) else {}
    guard_candidate = guard_candidate if isinstance(guard_candidate, dict) else {}
    return (
        type(seed_candidate.get("pid")) is int
        and type(guard_candidate.get("pid")) is int
        and guard_candidate.get("pid", 0) > 0
        and guard_candidate.get("pid") == seed_candidate.get("pid")
        and hash_matches(guard_candidate.get("sha256"), wrapper_sha)
        and same_declared_path(guard_candidate.get("path"), wrapper_path)
        and guard_payload.get("cancel_kept_exact_pid_alive") is True
    )


def pointer_value(value: Any, pointer: object) -> Any:
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        fail(f"invalid JSON pointer: {pointer!r}")
    current = value
    for raw_token in pointer.split("/")[1:]:
        token = raw_token.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict):
            if token not in current:
                fail(f"JSON pointer does not resolve: {pointer}")
            current = current[token]
        elif isinstance(current, list):
            if not token.isdigit() or int(token) >= len(current):
                fail(f"JSON pointer does not resolve: {pointer}")
            current = current[int(token)]
        else:
            fail(f"JSON pointer does not resolve: {pointer}")
    return current


def compact_observation(value: object) -> str:
    return " ".join(str(value if value is not None else "").split())


def exact_frozen_capture_coverage(
    receipt: dict[str, Any],
    capture: object,
    *,
    capture_index: int,
) -> tuple[list[str], list[dict[str, Any]]]:
    """Validate one current-UI capture and recompute every declared assertion.

    Assertion pointers are absolute from the outer method receipt.  Keeping the
    pointer in the signed receipt makes each family independently reviewable and
    prevents an index entry from silently borrowing a label from another capture.
    """

    if not isinstance(capture, dict):
        fail(f"frozen evidence capture {capture_index} is not an object")
    covers = capture.get("covers")
    assertions = capture.get("cover_assertions")
    labels = capture.get("observed_results_labels")
    if (
        not isinstance(covers, list)
        or not covers
        or not all(isinstance(family, str) and family for family in covers)
        or len(covers) != len(set(covers))
    ):
        fail(f"frozen evidence capture {capture_index} has invalid or duplicate covers")
    if not isinstance(assertions, list) or len(assertions) != len(covers):
        fail(f"frozen evidence capture {capture_index} must have one assertion per cover")
    if not isinstance(labels, dict):
        fail(f"frozen evidence capture {capture_index} lacks observed result labels")

    required_label_arrays = (
        "navigation",
        "visible_headings",
        "visible_result_table_ids",
        "table_titles",
        "table_headers",
        "table_rows",
        "chart_titles",
    )
    if not all(isinstance(labels.get(source), list) for source in required_label_arrays):
        fail(f"frozen evidence capture {capture_index} has an incomplete observation surface")
    if not compact_observation(labels.get("selected_result")) or not compact_observation(
        labels.get("document_tab")
    ):
        fail(f"frozen evidence capture {capture_index} lacks selected result identity")

    assertion_families: list[str] = []
    observation_pointers: list[str] = []
    normalized_assertions: list[dict[str, Any]] = []
    prefix = f"/evidence/{capture_index}/observed_results_labels/"
    table_evidence_used = False
    for assertion in assertions:
        if not isinstance(assertion, dict):
            fail(f"frozen evidence capture {capture_index} has a non-object assertion")
        family = assertion.get("family")
        source = assertion.get("source")
        matcher = assertion.get("matcher")
        expected = compact_observation(assertion.get("value"))
        pointer = assertion.get("observed_json_pointer")
        if (
            not isinstance(family, str)
            or family not in covers
            or source not in FROZEN_OBSERVATION_SOURCES
            or matcher not in FROZEN_OBSERVATION_MATCHERS
            or not expected
            or not isinstance(pointer, str)
            or not pointer.startswith(f"{prefix}{source}/")
        ):
            fail(f"frozen evidence capture {capture_index} has an invalid assertion contract")
        observed_raw = pointer_value(receipt, pointer)
        if isinstance(observed_raw, (dict, list)):
            fail(f"frozen assertion pointer must resolve to a scalar observation: {pointer}")
        observed = compact_observation(observed_raw)
        if not observed or compact_observation(assertion.get("observed_value")) != observed:
            fail(f"frozen assertion observed value is empty or does not match {pointer}")
        expected_folded = expected.casefold()
        observed_folded = observed.casefold()
        matched = (
            observed_folded == expected_folded
            if matcher == "exact"
            else len(expected) >= 4 and expected_folded in observed_folded
        )
        if not matched or assertion.get("passed") is not True:
            fail(f"frozen assertion did not recompute as passing: {family}")
        assertion_families.append(family)
        observation_pointers.append(pointer)
        table_evidence_used = table_evidence_used or source in FROZEN_TABLE_OBSERVATION_SOURCES
        normalized_assertions.append(copy.deepcopy(assertion))

    if assertion_families != covers or len(assertion_families) != len(set(assertion_families)):
        fail(f"frozen evidence capture {capture_index} assertion families do not exactly match covers")
    if len(observation_pointers) != len(set(observation_pointers)):
        fail(f"frozen evidence capture {capture_index} reuses an observed JSON pointer")
    if table_evidence_used and (
        not any(compact_observation(value) for value in labels["table_headers"])
        or not any(compact_observation(value) for value in labels["table_rows"])
    ):
        fail(f"frozen table evidence capture {capture_index} has empty headers or rows")
    return list(covers), normalized_assertions


def safe(name: object) -> bool:
    if not isinstance(name, str) or not name or "\\" in name or "\x00" in name or ":" in name:
        return False
    value = PurePosixPath(name)
    return not value.is_absolute() and value.as_posix() == name and all(part not in {"", ".", ".."} for part in value.parts)


def member_path(root: Path, member: object) -> Path:
    if not safe(member):
        fail(f"unsafe member: {member!r}")
    result = root.joinpath(*PurePosixPath(str(member)).parts).resolve()
    try:
        result.relative_to(root.resolve())
    except ValueError:
        fail(f"member escapes staging: {member}")
    if not result.is_file() or result.is_symlink():
        fail(f"member is missing, non-file, or symlink: {member}")
    return result


def files_below(root: Path) -> set[str]:
    if not root.is_dir():
        fail(f"staging directory is missing: {root}")
    result: set[str] = set()
    for item in root.rglob("*"):
        if item.is_symlink():
            fail(f"staging contains a symlink: {item}")
        if item.is_file():
            result.add(item.relative_to(root).as_posix())
    return result


def write_json_new(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def assert_sha(root: Path, ref: dict[str, Any], label: str) -> str:
    member = ref.get("member")
    declared = ref.get("sha256")
    path = member_path(root, member)
    if not isinstance(declared, str) or len(declared) != 64 or digest(path) != declared:
        fail(f"{label} SHA-256 mismatch: {member}")
    return str(member)


def validate_trusted_driver_provenance(
    collector: dict[str, Any],
    named_index: dict[str, Any],
    candidate_report: dict[str, Any],
    staging: Path,
    declared_members: list[object],
) -> None:
    contract = named_index.get("collector_contract")
    trusted_rows = contract.get("trusted_driver_suites") if isinstance(contract, dict) else None
    trusted_suites = {
        row.get("suite_id"): row.get("schema_version")
        for row in trusted_rows
        if isinstance(row, dict)
        and isinstance(row.get("suite_id"), str)
        and type(row.get("schema_version")) is int
    } if isinstance(trusted_rows, list) else {}
    if trusted_suites != TRUSTED_DRIVER_SUITES or len(trusted_rows or []) != len(trusted_suites):
        fail("named index trusted driver suite contract is not exact")

    provenance = collector.get("provenance")
    report_rows = provenance.get("trusted_driver_reports") if isinstance(provenance, dict) else None
    if not isinstance(report_rows, list) or not report_rows:
        fail("collector provenance has no trusted driver report inventory")
    if not exact_candidate_driver_inventory(candidate_report, report_rows):
        fail(
            "collector trusted driver inventory does not exactly match candidate report bindings"
        )
    seen_members: set[str] = set()
    seen_hashes: set[str] = set()
    seen_suites: set[object] = set()
    cross_count = 0
    declared_member_set = {member for member in declared_members if isinstance(member, str)}
    candidate_outcomes_raw = candidate_report.get("outcomes")
    candidate_outcomes = {
        outcome.get("name"): outcome
        for outcome in candidate_outcomes_raw
        if isinstance(outcome, dict) and isinstance(outcome.get("name"), str)
    } if isinstance(candidate_outcomes_raw, list) else {}
    candidate_source_commit = candidate_report.get("candidate_build_source_commit")
    for row in report_rows:
        if not isinstance(row, dict) or set(row) != {
            "candidate",
            "role",
            "suite_id",
            "schema_version",
            "member",
            "sha256",
            "supplied_named_observations",
            "phase_reports",
        }:
            fail("collector trusted driver provenance row has an inexact shape")
        suite_id = row.get("suite_id")
        member = row.get("member")
        declared_hash = row.get("sha256")
        if (
            row.get("candidate") not in {"portable", "installed"}
            or not isinstance(row.get("role"), str)
            or not row.get("role")
            or not isinstance(member, str)
            or member not in declared_member_set
            or not isinstance(declared_hash, str)
            or len(declared_hash) != 64
            or not isinstance(suite_id, str)
            or suite_id not in trusted_suites
            or type(row.get("schema_version")) is not int
            or row.get("schema_version") != trusted_suites.get(suite_id)
        ):
            fail("collector trusted driver provenance row is incomplete")
        report_path = member_path(staging, member)
        if digest(report_path) != declared_hash:
            fail(f"trusted driver report SHA-256 mismatch: {member}")
        payload = load(report_path)
        observations = payload.get("named_evidence_observations")
        observations = [] if observations is None else observations
        if not (
            payload.get("suite_id") == suite_id
            and exact_schema_version(payload, row.get("schema_version"))
            and driver_report_passed(payload)
            and isinstance(observations, list)
            and row.get("supplied_named_observations") == len(observations)
        ):
            fail(f"trusted driver report lacks exact zero console errors: {member}")
        if str(member) in seen_members or declared_hash in seen_hashes:
            fail("collector trusted driver report inventory reuses a member or hash")
        seen_members.add(str(member))
        seen_hashes.add(declared_hash)
        seen_suites.add(suite_id)

        phase_refs = row.get("phase_reports")
        if suite_id != CROSS_WRAPPER_SUITE:
            if phase_refs != []:
                fail("only the cross wrapper may bind cross phase reports")
            continue
        cross_count += 1
        if not isinstance(phase_refs, list):
            fail("cross wrapper phase report provenance is not an array")
        phase_records: list[dict[str, Any]] = []
        for phase_ref in phase_refs:
            if not isinstance(phase_ref, dict) or set(phase_ref) != {
                "phase",
                "member",
                "sha256",
                "suite_id",
                "schema_version",
                "renderer_attached",
            }:
                fail("cross phase provenance row has an inexact shape")
            phase_member = phase_ref.get("member")
            phase_hash = phase_ref.get("sha256")
            if (
                not isinstance(phase_member, str)
                or phase_member not in declared_member_set
                or not isinstance(phase_hash, str)
                or len(phase_hash) != 64
            ):
                fail("cross phase provenance row is incomplete")
            phase_path = member_path(staging, phase_member)
            if digest(phase_path) != phase_hash:
                fail(f"cross phase report SHA-256 mismatch: {phase_member}")
            phase_records.append({**phase_ref, "payload": load(phase_path)})
        expected_outcome = candidate_outcomes.get(str(row.get("candidate")), {})
        if not exact_cross_phase_payloads(
            payload,
            phase_records,
            expected_candidate_sha256=expected_outcome.get("executable_sha256"),
            expected_candidate_path=expected_outcome.get("executable"),
            expected_product_version=expected_outcome.get("product_version"),
            expected_source_commit=candidate_source_commit,
        ):
            fail("cross wrapper does not bind the exact nine clean phase reports")

    if seen_suites != set(TRUSTED_DRIVER_SUITES) or cross_count != 1:
        fail("collector provenance does not cover every exact trusted driver suite")


def derive_frozen(template: dict[str, Any], staging: Path, aggregate: dict[str, Any]) -> dict[str, Any]:
    if not (exact_schema_version(aggregate, 1) and aggregate.get("suite_id") == FROZEN_SUITE and aggregate.get("target_release") == TARGET and aggregate.get("status") == "passed" and aggregate.get("failures") == [] and exact_empty_console_errors(aggregate)):
        fail("frozen aggregate is not a passing exact 2.55 crawler report")
    artifacts = aggregate.get("method_receipts")
    methods = template.get("methods")
    if not isinstance(artifacts, list) or len(artifacts) != 18 or not isinstance(methods, list) or len(methods) != 18:
        fail("frozen aggregate/template must each contain exactly 18 methods")
    by_kind = {row.get("kind"): row for row in methods if isinstance(row, dict)}
    if len(by_kind) != 18:
        fail("frozen template method kinds are not unique")
    if template.get("schema_version") != 4:
        fail("frozen index template must use the incompatible multi-capture v4 schema")
    proposal = copy.deepcopy(template)
    proposed_by_kind = {row["kind"]: row for row in proposal["methods"]}
    receipt_kinds: list[str] = []
    for artifact in artifacts:
        if not isinstance(artifact, dict):
            fail("frozen receipt artifact is not an object")
        receipt_member = assert_sha(staging, artifact, "frozen receipt")
        receipt = load(member_path(staging, receipt_member))
        kind = receipt.get("method_kind")
        declared = by_kind.get(kind)
        captures = receipt.get("evidence")
        if not (
            receipt.get("schema_version") == 2
            and receipt.get("suite_id") == FROZEN_SUITE
            and receipt.get("target_release") == TARGET
            and receipt.get("status") == "verified_current_ui_capture"
            and isinstance(declared, dict)
            and artifact.get("method_kind") == kind
            and artifact.get("status") == receipt.get("status")
            and isinstance(captures, list)
            and captures
        ):
            fail(f"invalid frozen method receipt: {kind!r}")
        receipt_kinds.append(kind)
        required = declared.get("representative_results")
        if (
            not isinstance(required, list)
            or not required
            or not all(isinstance(family, str) and family for family in required)
            or len(required) != len(set(required))
        ):
            fail(f"frozen template has invalid representative results: {kind}")
        covered: list[str] = []
        proposed_evidence: list[dict[str, Any]] = []
        receipt_observation_pointers: list[str] = []
        for capture_index, capture in enumerate(captures):
            covers, assertions = exact_frozen_capture_coverage(
                receipt, capture, capture_index=capture_index
            )
            assert isinstance(capture, dict)
            identity_verification = capture.get("identity_verification")
            source_receipt = capture.get("source_receipt")
            if (
                capture.get("status") != "verified_current_ui_capture"
                or not isinstance(identity_verification, dict)
                or identity_verification.get("passed") is not True
                or not isinstance(source_receipt, dict)
                or not (
                    source_receipt.get("declared_identity_directly_bound") is True
                    or source_receipt.get("identity_recovered_from_archive") is True
                )
                or (
                    kind == "pls_posthoc_technical_minimum_sample_size"
                    and source_receipt.get("declared_identity_directly_bound") is not True
                )
            ):
                fail(f"frozen capture is not identity/provenance verified: {kind} #{capture_index}")
            covered.extend(covers)
            receipt_observation_pointers.extend(
                assertion["observed_json_pointer"] for assertion in assertions
            )
            archive_member = assert_sha(
                staging, capture.get("archive") or {}, f"{kind} capture {capture_index} archive"
            )
            screenshot_member = assert_sha(
                staging,
                capture.get("screenshot") or {},
                f"{kind} capture {capture_index} screenshot",
            )
            assert_sha(
                staging,
                source_receipt,
                f"{kind} capture {capture_index} source receipt",
            )
            identity = capture.get("declared_identity")
            if (
                not isinstance(identity, dict)
                or identity.get("type")
                not in {"canonical_result_document_id", "schema5_result_run_id"}
                or not isinstance(identity.get("value"), str)
                or not identity["value"]
            ):
                fail(f"frozen capture lacks declared result identity: {kind} #{capture_index}")
            evidence_pointer = f"/evidence/{capture_index}"
            proposed_evidence.append(
                {
                    "method_kind": kind,
                    "canonical_result_id": identity["value"],
                    "covers": covers,
                    "cover_assertions": assertions,
                    "archive": {
                        "member": archive_member,
                        "sha256": capture["archive"]["sha256"],
                    },
                    "screenshot": {
                        "member": screenshot_member,
                        "sha256": capture["screenshot"]["sha256"],
                    },
                    "receipt": {
                        "member": receipt_member,
                        "sha256": artifact["sha256"],
                        "method_kind_json_pointer": "/method_kind",
                        "canonical_result_id_json_pointer": f"{evidence_pointer}/declared_identity/value",
                        "evidence_json_pointer": evidence_pointer,
                    },
                }
            )
        if len(receipt_observation_pointers) != len(set(receipt_observation_pointers)):
            fail(f"frozen receipt reuses an observed JSON pointer: {kind}")
        if set(covered) != set(required) or len(covered) != len(set(covered)):
            fail(f"frozen receipt does not exactly cover declared result families: {kind}")
        proposed_by_kind[kind]["status"] = "verified"
        proposed_by_kind[kind]["evidence"] = proposed_evidence
    if len(receipt_kinds) != len(set(receipt_kinds)) or set(proposed_by_kind) != set(receipt_kinds):
        fail("frozen receipts do not cover each public method exactly once")
    proposal["status"] = "verified"
    return proposal


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a new deterministic QuickPLS 2.55 evidence ZIP from passing staged evidence.")
    parser.add_argument("--named-staging", required=True, type=Path)
    parser.add_argument("--named-collector-report", required=True, type=Path)
    parser.add_argument("--named-collected-index", required=True, type=Path)
    parser.add_argument("--frozen-staging", required=True, type=Path)
    parser.add_argument("--frozen-aggregate-report", required=True, type=Path)
    parser.add_argument("--frozen-index-template", type=Path, default=ROOT / "validation/v255_frozen_result_archive_index.json")
    parser.add_argument("--bundle-manifest-template", type=Path, default=ROOT / "validation/v255_evidence_bundle_manifest.json")
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    named_root, frozen_root, output = args.named_staging.resolve(), args.frozen_staging.resolve(), args.output_dir.resolve()
    if output.exists():
        fail(f"refusing to overwrite output directory: {output}")
    output.mkdir(parents=True, exist_ok=False)
    collector = load(args.named_collector_report.resolve())
    named_index = load(args.named_collected_index.resolve())
    summary = collector.get("summary", {})
    entries = named_index.get("entries", [])
    normal_collection = (
        collector.get("status") == "passed"
        and summary.get("verified") == 55
        and summary.get("waived") == 0
        and named_index.get("status") == "verified"
        and all(isinstance(entry, dict) and entry.get("status") == "verified" for entry in entries)
    )
    waived_collection = (
        collector.get("status") == "passed_with_waiver"
        and summary.get("verified") == 54
        and summary.get("waived") == 1
        and exact_population_status(entries, named_index.get("status"))
    )
    if not (collector.get("schema_version") == 1 and collector.get("suite_id") == COLLECTOR_SUITE and collector.get("target_release") == TARGET and collector.get("passed") is True and collector.get("failures") == [] and summary.get("collected") == 55 and len(entries) == 55 and (normal_collection or waived_collection)):
        fail("named collector/index are not an exact 55-case collection with zero waivers or the one approved DPI waiver")
    named_members = collector.get("bundle_members")
    if not isinstance(named_members, list) or len(named_members) != len(set(named_members)):
        fail("named collector has no unique exact member inventory")
    for member in named_members:
        member_path(named_root, member)
    candidate_member = collector.get("sources", {}).get("candidate_report_member")
    candidate_path = member_path(named_root, candidate_member)
    candidate = load(candidate_path)
    candidate_hash = collector.get("sources", {}).get("candidate_report_sha256")
    if not isinstance(candidate_hash, str) or digest(candidate_path) != candidate_hash:
        fail("named collector does not hash-bind its candidate report")
    validate_trusted_driver_provenance(
        collector, named_index, candidate, named_root, named_members
    )
    expected_named_files = set(named_members) | {args.named_collected_index.resolve().relative_to(named_root).as_posix()}
    if files_below(named_root) != expected_named_files:
        fail("named staging contains missing or undeclared files")

    if waived_collection:
        waived_entries = [
            entry
            for entry in entries
            if isinstance(entry, dict) and entry.get("status") == "waived"
        ]
        waived_entry = waived_entries[0]
        receipt_ref = waived_entry.get("receipt", {})
        receipt_path = member_path(named_root, receipt_ref.get("member"))
        if digest(receipt_path) != receipt_ref.get("sha256"):
            fail("waived receipt does not match its collected-index SHA-256")
        receipt = load(receipt_path)
        source_ref = receipt.get("source_report", {})
        source_path = member_path(named_root, source_ref.get("member"))
        if digest(source_path) != source_ref.get("sha256"):
            fail("waived source report does not match its collected receipt SHA-256")
        source = load(source_path)
        observation = pointer_value(source, source_ref.get("json_pointer"))
        screenshot_ref = waived_entry.get("screenshot", {})
        screenshot_path = member_path(named_root, screenshot_ref.get("member"))
        candidate_waivers = candidate.get("release_waivers")
        if not (
            canonical_digest(observation) == source_ref.get("observation_sha256")
            and isinstance(candidate_waivers, list)
            and len(candidate_waivers) == 1
            and exact_release_waiver_matches_observation(
                candidate_waivers[0], observation
            )
            and exact_cross_report_waiver_binding(source, observation)
            and exact_case_waiver_receipt_matches_observation(receipt, observation)
            and receipt.get("candidate_report", {}).get("member") == candidate_member
            and receipt.get("candidate_report", {}).get("sha256") == candidate_hash
            and receipt.get("screenshot", {}).get("member")
            == screenshot_ref.get("member")
            and receipt.get("screenshot", {}).get("sha256")
            == screenshot_ref.get("sha256")
            and digest(screenshot_path) == screenshot_ref.get("sha256")
        ):
            fail(
                "waiver is not exactly bound across candidate, cross-report DPI, source observation, receipt, and screenshot"
            )
    elif not (
        candidate.get("qualification_status") == "passed"
        and candidate.get("release_waivers") == []
    ):
        fail("strict named evidence requires an exact zero-waiver candidate report")

    frozen_aggregate = load(args.frozen_aggregate_report.resolve())
    frozen_index = derive_frozen(load(args.frozen_index_template.resolve()), frozen_root, frozen_aggregate)
    before = frozen_aggregate.get("staged_members_before_manifest")
    aggregate_member = args.frozen_aggregate_report.resolve().relative_to(frozen_root).as_posix()
    if not isinstance(before, list) or files_below(frozen_root) != set(before) | {aggregate_member}:
        fail("frozen staging does not exactly match its aggregate member inventory")
    frozen_members = sorted(set(before) | {aggregate_member})
    names = sorted(set(named_members) | set(frozen_members))
    if len(names) != len(named_members) + len(frozen_members):
        fail("named and frozen staged members collide")

    zip_path = output / "QuickPLS-2.55-evidence.zip"
    with zipfile.ZipFile(zip_path, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=True) as archive:
        for name in names:
            source = member_path(named_root if name in set(named_members) else frozen_root, name)
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | 0o644) << 16
            info.flag_bits = 0
            archive.writestr(info, source.read_bytes(), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    with zipfile.ZipFile(zip_path, "r") as archive:
        if archive.testzip() is not None or [row.filename for row in archive.infolist()] != names or any(row.flag_bits & 1 for row in archive.infolist()):
            fail("new deterministic ZIP failed CRC/member/encryption verification")
        infos = archive.infolist()
        compressed = sum(row.compress_size for row in infos)
        uncompressed = sum(row.file_size for row in infos)

    manifest = copy.deepcopy(load(args.bundle_manifest_template.resolve()))
    collector_member = args.named_collector_report.resolve().relative_to(named_root).as_posix()
    candidate_member = collector["sources"]["candidate_report_member"]
    schema_member = collector["sources"]["observation_schema_member"]
    named_manifest_member = collector["sources"]["named_case_manifest_member"]
    manifest["status"] = "verified_with_waiver" if waived_collection else "verified"
    manifest["bundle"].update({"sha256": digest(zip_path), "member_count": len(names), "ordered_member_names_sha256": hashlib.sha256("\n".join(names).encode()).hexdigest(), "compressed_bytes": compressed, "uncompressed_bytes": uncompressed})
    manifest["named_evidence"].update({
        "collector_report": {"member": collector_member, "sha256": digest(member_path(named_root, collector_member))},
        "candidate_report": {"member": candidate_member, "sha256": collector["sources"]["candidate_report_sha256"]},
        "observation_schema": {"member": schema_member, "sha256": collector["sources"]["observation_schema_sha256"]},
        "named_case_manifest": {"member": named_manifest_member, "sha256": collector["sources"]["named_case_manifest_sha256"]},
        "source_commit": collector["provenance"]["source_commit"],
        "candidate_executables": collector["provenance"]["candidate_executables"],
        "approved_release_waiver": DPI_WAIVER_MANIFEST_DECLARATION,
    })
    write_json_new(output / "v255_named_evidence_index.proposed.json", named_index)
    write_json_new(output / "v255_frozen_result_archive_index.proposed.json", frozen_index)
    write_json_new(output / "v255_evidence_bundle_manifest.proposed.json", manifest)
    report = {"schema_version": 1, "suite_id": "quickpls_v255_evidence_bundle_builder_v1", "target_release": TARGET, "passed": True, "qualification_status": "passed_with_waiver" if waived_collection else "passed", "approved_release_waiver": DPI_WAIVER_MANIFEST_DECLARATION if waived_collection else None, "bundle": str(zip_path), "bundle_sha256": digest(zip_path), "member_count": len(names), "compressed_bytes": compressed, "uncompressed_bytes": uncompressed, "proposals": {"named_index": str(output / "v255_named_evidence_index.proposed.json"), "frozen_index": str(output / "v255_frozen_result_archive_index.proposed.json"), "bundle_manifest": str(output / "v255_evidence_bundle_manifest.proposed.json")}, "apply_rule": "Review then atomically replace the three source contracts; never edit hashes by hand."}
    write_json_new(output / "v255_evidence_bundle_builder.json", report)
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
