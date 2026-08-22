from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import re
import struct
import sys
import zipfile
import zlib
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MATRIX = ROOT / "validation" / "v255_method_evidence_matrix.json"
DEFAULT_INDEX = ROOT / "validation" / "v255_named_evidence_index.json"
DEFAULT_MANIFEST = ROOT / "validation" / "v255_evidence_bundle_manifest.json"
DEFAULT_OBSERVATION_SCHEMA = ROOT / "validation" / "v255_named_evidence_observation.schema.json"
DEFAULT_NAMED_CASE_MANIFEST = ROOT / "validation" / "v255_named_case_manifest.json"
DEFAULT_OUTPUT = ROOT / "validation" / "results" / "v255_named_evidence_verifier.json"
TARGET_RELEASE = "2.55.0"
FROZEN_CASE_SET_SHA256 = "98ed24bc3d4453cec21768b3c084c916c88acfb831baefbd737d01749d3e105f"
COLLECTOR_SUITE = "quickpls_v255_named_evidence_collector_v1"
CASE_RECEIPT_SUITE = "quickpls_v255_named_evidence_case_receipt_v1"
CANDIDATE_SUITE = "quickpls_v255_installed_portable_smoke_v3"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
WINDOWS_RESERVED = re.compile(r"^(?:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)", re.IGNORECASE)
MAX_SCREENSHOT_BYTES = 64 * 1024 * 1024
MAX_RECEIPT_BYTES = 16 * 1024 * 1024
MAX_SOURCE_REPORT_BYTES = 32 * 1024 * 1024
MAX_BUNDLE_MEMBERS = 512
MAX_BUNDLE_UNCOMPRESSED_BYTES = 1024 * 1024 * 1024
MAX_COMPRESSION_RATIO = 200


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"JSON root must be an object: {path}")
    return value


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_path(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def validate_png_bytes(value: bytes) -> tuple[bool, int | None, int | None]:
    if not (45 <= len(value) <= MAX_SCREENSHOT_BYTES) or not value.startswith(PNG_SIGNATURE):
        return False, None, None
    offset = len(PNG_SIGNATURE)
    chunk_index = 0
    width: int | None = None
    height: int | None = None
    while offset < len(value):
        if offset + 12 > len(value):
            return False, None, None
        length = struct.unpack(">I", value[offset : offset + 4])[0]
        chunk_type = value[offset + 4 : offset + 8]
        end = offset + 12 + length
        if end > len(value):
            return False, None, None
        chunk_data = value[offset + 8 : offset + 8 + length]
        expected_crc = struct.unpack(">I", value[offset + 8 + length : end])[0]
        if zlib.crc32(chunk_type + chunk_data) & 0xFFFFFFFF != expected_crc:
            return False, None, None
        if chunk_index == 0:
            if chunk_type != b"IHDR" or length != 13:
                return False, None, None
            width, height = struct.unpack(">II", chunk_data[:8])
            if width <= 0 or height <= 0:
                return False, None, None
        if chunk_type == b"IEND":
            return length == 0 and end == len(value), width, height
        offset = end
        chunk_index += 1
    return False, None, None


def driver_report_passed(payload: dict[str, Any]) -> bool:
    return (
        payload.get("target_release") == TARGET_RELEASE
        and (payload.get("passed") is True or payload.get("status") in {"passed", "verified"})
    )


def expected_operation(entry: dict[str, Any], contract: dict[str, Any]) -> str | None:
    operations = contract.get("operation_by_group")
    return operations.get(entry.get("group")) if isinstance(operations, dict) else None


def expected_assertion_id(entry: dict[str, Any], operation: object) -> str | None:
    return f"{operation}:{entry.get('id')}" if isinstance(operation, str) and operation else None


def expected_candidate_name(entry: dict[str, Any], contract: dict[str, Any]) -> str | None:
    selection = contract.get("candidate_selection")
    if not isinstance(selection, dict):
        return None
    overrides = selection.get("overrides")
    if not isinstance(overrides, dict):
        return None
    value = overrides.get(entry.get("id"), selection.get("default"))
    return value if value in {"portable", "installed"} else None


def expected_cases(matrix: dict[str, Any]) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    cross = matrix.get("cross_method_evidence")
    if not isinstance(cross, dict):
        raise ValueError("matrix cross_method_evidence must be an object")
    for group in ("imports", "exports", "persistence", "accessibility", "observability", "packaged"):
        declaration = cross.get(group)
        if not isinstance(declaration, dict):
            raise ValueError(f"matrix cross-method group is missing: {group}")
        required = declaration.get("required")
        evidence = declaration.get("evidence")
        if not isinstance(required, list) or not all(isinstance(case, str) and case for case in required):
            raise ValueError(f"matrix cross-method required cases are invalid: {group}")
        if not isinstance(evidence, list):
            raise ValueError(f"matrix cross-method evidence is invalid: {group}")
        declarations = {
            item.get("case"): item
            for item in evidence
            if isinstance(item, dict) and isinstance(item.get("case"), str)
        }
        if len(declarations) != len(evidence) or set(declarations) != set(required):
            raise ValueError(f"matrix cross-method declarations do not exactly cover: {group}")
        if any(item.get("status") not in {"ready", "post_candidate"} for item in declarations.values()):
            raise ValueError(f"matrix evidence statuses must remain ready/post_candidate: {group}")
        rows.extend(
            {
                "id": f"cross_method:{group}:{case}",
                "scope": "cross_method",
                "group": group,
                "case": case,
            }
            for case in required
        )

    specialized = matrix.get("specialized_result_evidence")
    if not isinstance(specialized, dict):
        raise ValueError("matrix specialized_result_evidence must be an object")
    required = specialized.get("required")
    evidence = specialized.get("evidence")
    if not isinstance(required, list) or not all(isinstance(case, str) and case for case in required):
        raise ValueError("matrix specialized required cases are invalid")
    if not isinstance(evidence, list):
        raise ValueError("matrix specialized evidence is invalid")
    declarations = {
        item.get("case"): item
        for item in evidence
        if isinstance(item, dict) and isinstance(item.get("case"), str)
    }
    if len(declarations) != len(evidence) or set(declarations) != set(required):
        raise ValueError("matrix specialized declarations do not exactly cover required cases")
    if any(item.get("status") not in {"ready", "post_candidate"} for item in declarations.values()):
        raise ValueError("matrix specialized statuses must remain ready/post_candidate")
    rows.extend(
        {
            "id": f"specialized_result:{case}",
            "scope": "specialized_result",
            "group": "specialized_result_evidence",
            "case": case,
        }
        for case in required
    )
    return rows


def safe_zip_member(member: object, *, file_only: bool = False) -> bool:
    if not isinstance(member, str) or not member or "\\" in member or "\x00" in member:
        return False
    if member.startswith("/") or ":" in member:
        return False
    normalized = member[:-1] if member.endswith("/") else member
    if not normalized:
        return False
    path = PurePosixPath(normalized)
    return (
        path.as_posix() == normalized
        and not path.is_absolute()
        and all(part not in {"", ".", ".."} for part in path.parts)
        and all(part == part.rstrip(" .") and WINDOWS_RESERVED.match(part) is None for part in path.parts)
        and (not file_only or not member.endswith("/"))
    )


def pointer_value(payload: Any, pointer: object) -> Any:
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        raise ValueError("case binding must be a non-root JSON pointer")
    current = payload
    for encoded in pointer[1:].split("/"):
        if re.search(r"~(?:[^01]|$)", encoded):
            raise ValueError(f"invalid JSON pointer escape: {pointer}")
        token = encoded.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict):
            if token not in current:
                raise KeyError(pointer)
            current = current[token]
        elif isinstance(current, list):
            if not token.isdigit():
                raise KeyError(pointer)
            index = int(token)
            if index < 0 or index >= len(current):
                raise KeyError(pointer)
            current = current[index]
        else:
            raise KeyError(pointer)
    return current


def validate_index_shape(
    matrix: dict[str, Any], index: dict[str, Any], publication: bool
) -> tuple[list[dict[str, str]], list[dict[str, Any]], list[str], dict[str, bool]]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    expected = expected_cases(matrix)
    raw_entries = index.get("entries")
    entries = [entry for entry in raw_entries if isinstance(entry, dict)] if isinstance(raw_entries, list) else []
    expected_by_id = {row["id"]: row for row in expected}
    actual_by_id = {
        str(entry.get("id", "")): entry
        for entry in entries
        if isinstance(entry.get("id"), str)
    }

    checks["index_schema_and_release_are_exact"] = (
        index.get("schema_version") == 1 and index.get("target_release") == TARGET_RELEASE
    )
    checks["matrix_declares_exactly_29_cross_and_26_specialized_cases"] = (
        sum(row["scope"] == "cross_method" for row in expected) == 29
        and sum(row["scope"] == "specialized_result" for row in expected) == 26
        and len(expected) == 55
    )
    checks["ordered_case_identity_set_matches_frozen_2_55_contract"] = (
        sha256_bytes("\n".join(row["id"] for row in expected).encode("utf-8")) == FROZEN_CASE_SET_SHA256
        and index.get("publication_contract", {}).get("ordered_case_id_set_sha256") == FROZEN_CASE_SET_SHA256
    )
    checks["index_has_55_unique_entries"] = len(entries) == len(actual_by_id) == 55
    checks["index_exactly_covers_matrix_named_cases"] = set(actual_by_id) == set(expected_by_id)
    checks["entry_scope_group_and_case_match_matrix"] = checks["index_exactly_covers_matrix_named_cases"] and all(
        all(actual_by_id[case_id].get(field) == expected_by_id[case_id][field] for field in ("scope", "group", "case"))
        for case_id in expected_by_id
    )
    checks["entry_statuses_are_pending_or_verified"] = all(
        entry.get("status") in {"pending", "verified"} for entry in entries
    )
    checks["binding_expected_values_equal_entry_ids"] = all(
        isinstance(entry.get("receipt"), dict)
        and isinstance(entry["receipt"].get("binding"), dict)
        and entry["receipt"]["binding"].get("expected_value") == entry.get("id")
        for entry in entries
    )
    collector_contract = index.get("collector_contract")
    trusted_suites = collector_contract.get("trusted_driver_suites") if isinstance(collector_contract, dict) else None
    trusted_suite_map = {
        row.get("suite_id"): row.get("schema_version")
        for row in trusted_suites
        if isinstance(row, dict)
        and isinstance(row.get("suite_id"), str)
        and isinstance(row.get("schema_version"), int)
    } if isinstance(trusted_suites, list) else {}
    expected_groups = {
        "imports",
        "exports",
        "persistence",
        "accessibility",
        "observability",
        "packaged",
        "specialized_result_evidence",
    }
    operations = collector_contract.get("operation_by_group") if isinstance(collector_contract, dict) else None
    candidate_selection = collector_contract.get("candidate_selection") if isinstance(collector_contract, dict) else None
    checks["collector_contract_has_exact_fail_closed_identity"] = (
        isinstance(collector_contract, dict)
        and collector_contract.get("schema_version") == 1
        and collector_contract.get("suite_id") == COLLECTOR_SUITE
        and collector_contract.get("case_receipt_suite_id") == CASE_RECEIPT_SUITE
        and collector_contract.get("candidate_report_suite_id") == CANDIDATE_SUITE
        and collector_contract.get("candidate_report_stage") == "source"
        and collector_contract.get("driver_observation_field") == "named_evidence_observations"
        and isinstance(operations, dict)
        and set(operations) == expected_groups
        and all(isinstance(value, str) and value for value in operations.values())
        and isinstance(candidate_selection, dict)
        and candidate_selection.get("default") == "portable"
        and candidate_selection.get("overrides") == {"cross_method:packaged:installed candidate": "installed"}
        and len(trusted_suite_map) == len(trusted_suites or [])
        and trusted_suite_map.get("quickpls_v255_live_calculation_lifecycle_smoke_v1") == 1
        and trusted_suite_map.get("quickpls_v255_method_evidence_crawler_v2") == 2
        and trusted_suite_map.get("quickpls_v255_frozen_archive_reopen_crawler_v1") == 1
        and trusted_suite_map.get("quickpls_v255_named_case_driver_v1") == 1
    )
    publication_contract = index.get("publication_contract")
    checks["publication_contract_requires_collector_candidate_source_and_unique_artifacts"] = (
        isinstance(publication_contract, dict)
        and publication_contract.get("case_receipt_suite_id") == CASE_RECEIPT_SUITE
        and publication_contract.get("collector_report_suite_id") == COLLECTOR_SUITE
        and publication_contract.get("candidate_report_suite_id") == CANDIDATE_SUITE
        and publication_contract.get("source_report_and_pointer_binding_required") is True
        and publication_contract.get("candidate_sha256_and_source_commit_binding_required") is True
        and publication_contract.get("unique_case_artifacts_required") is True
        and publication_contract.get("executed_named_case_manifest_hash_binding_required") is True
    )
    checks["pending_and_verified_rows_have_no_partial_artifact_state"] = all(
        (
            entry.get("status") == "pending"
            and isinstance(entry.get("screenshot"), dict)
            and entry["screenshot"].get("member") is None
            and entry["screenshot"].get("sha256") is None
            and isinstance(entry.get("receipt"), dict)
            and entry["receipt"].get("member") is None
            and entry["receipt"].get("sha256") is None
            and isinstance(entry["receipt"].get("binding"), dict)
            and entry["receipt"]["binding"].get("json_pointer") is None
        )
        or (
            entry.get("status") == "verified"
            and isinstance(entry.get("screenshot"), dict)
            and safe_zip_member(entry["screenshot"].get("member"), file_only=True)
            and isinstance(entry["screenshot"].get("sha256"), str)
            and SHA256_RE.fullmatch(entry["screenshot"]["sha256"]) is not None
            and isinstance(entry.get("receipt"), dict)
            and safe_zip_member(entry["receipt"].get("member"), file_only=True)
            and isinstance(entry["receipt"].get("sha256"), str)
            and SHA256_RE.fullmatch(entry["receipt"]["sha256"]) is not None
            and isinstance(entry["receipt"].get("binding"), dict)
            and entry["receipt"]["binding"].get("json_pointer") == "/case_id"
        )
        for entry in entries
    )
    checks["index_collection_status_matches_rows"] = (
        (entries and all(entry.get("status") == "verified" for entry in entries) and index.get("status") == "verified")
        or (any(entry.get("status") == "pending" for entry in entries) and index.get("status") == "pending_collection")
    )
    checks["publication_index_is_fully_verified"] = (
        not publication
        or (
            index.get("status") == "verified"
            and len(entries) == 55
            and all(entry.get("status") == "verified" for entry in entries)
        )
    )
    for name, passed in checks.items():
        if not passed:
            failures.append(name)
    return expected, entries, failures, checks


def verify_publication_entries(
    entries: list[dict[str, Any]],
    bundle: Path,
    manifest: dict[str, Any],
    matrix_hash: str,
    index_hash: str,
    observation_schema_hash: str,
    named_case_manifest_hash: str,
    collector_contract: dict[str, Any],
) -> tuple[list[dict[str, Any]], list[str], dict[str, bool], str, dict[str, Any]]:
    failures: list[str] = []
    case_reports: list[dict[str, Any]] = []
    provenance: dict[str, Any] = {}
    bundle_hash = sha256_path(bundle)
    bundle_declaration = manifest.get("bundle") if isinstance(manifest.get("bundle"), dict) else {}
    named_manifest = manifest.get("named_evidence") if isinstance(manifest.get("named_evidence"), dict) else {}
    expected_bundle_hash = bundle_declaration.get("sha256")
    collector_ref = named_manifest.get("collector_report") if isinstance(named_manifest.get("collector_report"), dict) else {}
    candidate_ref = named_manifest.get("candidate_report") if isinstance(named_manifest.get("candidate_report"), dict) else {}
    observation_schema_ref = named_manifest.get("observation_schema") if isinstance(named_manifest.get("observation_schema"), dict) else {}
    named_case_manifest_ref = named_manifest.get("named_case_manifest") if isinstance(named_manifest.get("named_case_manifest"), dict) else {}
    candidate_digests = named_manifest.get("candidate_executables") if isinstance(named_manifest.get("candidate_executables"), dict) else {}
    checks: dict[str, bool] = {
        "bundle_manifest_is_verified_for_2_55": manifest.get("status") == "verified"
        and manifest.get("target_release") == TARGET_RELEASE,
        "bundle_manifest_sha256_is_lowercase": isinstance(expected_bundle_hash, str)
        and SHA256_RE.fullmatch(expected_bundle_hash) is not None,
        "bundle_sha256_matches_manifest": isinstance(expected_bundle_hash, str)
        and bundle_hash == expected_bundle_hash,
        "named_evidence_manifest_declares_exact_suites": (
            named_manifest.get("collector_suite_id") == COLLECTOR_SUITE
            and named_manifest.get("case_receipt_suite_id") == CASE_RECEIPT_SUITE
            and named_manifest.get("source_reports_are_bundle_members") is True
            and named_manifest.get("unique_case_receipt_and_screenshot_bytes") is True
            and named_manifest.get("unique_source_report_pointer_per_case") is True
        ),
        "named_evidence_manifest_declares_candidate_provenance": (
            isinstance(named_manifest.get("source_commit"), str)
            and COMMIT_RE.fullmatch(named_manifest["source_commit"]) is not None
            and set(candidate_digests) == {"portable", "installed"}
            and all(isinstance(value, str) and SHA256_RE.fullmatch(value) is not None for value in candidate_digests.values())
        ),
        "collector_and_candidate_report_manifest_refs_are_safe_and_hashed": (
            safe_zip_member(collector_ref.get("member"), file_only=True)
            and isinstance(collector_ref.get("sha256"), str)
            and SHA256_RE.fullmatch(collector_ref["sha256"]) is not None
            and safe_zip_member(candidate_ref.get("member"), file_only=True)
            and isinstance(candidate_ref.get("sha256"), str)
            and SHA256_RE.fullmatch(candidate_ref["sha256"]) is not None
            and safe_zip_member(observation_schema_ref.get("member"), file_only=True)
            and isinstance(observation_schema_ref.get("sha256"), str)
            and SHA256_RE.fullmatch(observation_schema_ref["sha256"]) is not None
            and safe_zip_member(named_case_manifest_ref.get("member"), file_only=True)
            and named_case_manifest_ref.get("sha256") == named_case_manifest_hash
        ),
    }
    try:
        archive = zipfile.ZipFile(bundle, "r")
    except (OSError, zipfile.BadZipFile) as error:
        failures.append(f"evidence_bundle_is_not_a_readable_zip:{error}")
        checks["bundle_members_are_unique_safe_and_bounded"] = False
        failures.extend(name for name, passed in checks.items() if not passed)
        return case_reports, list(dict.fromkeys(failures)), checks, bundle_hash, provenance

    with archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        info_by_name = {info.filename: info for info in infos}
        total_compressed = sum(info.compress_size for info in infos if not info.is_dir())
        total_uncompressed = sum(info.file_size for info in infos if not info.is_dir())
        ordered_names_hash = sha256_bytes("\n".join(sorted(names)).encode("utf-8"))
        checks["bundle_members_are_unique_safe_and_bounded"] = (
            0 < len(names) <= MAX_BUNDLE_MEMBERS
            and len(names) == len({name.casefold() for name in names})
            and all(safe_zip_member(name) for name in names)
            and total_uncompressed <= MAX_BUNDLE_UNCOMPRESSED_BYTES
            and all((info.flag_bits & 0x1) == 0 for info in infos)
            and all(
                info.is_dir()
                or info.file_size == 0
                or (
                    info.compress_size > 0
                    and info.file_size <= info.compress_size * MAX_COMPRESSION_RATIO
                )
                for info in infos
            )
        )
        checks["bundle_member_inventory_matches_manifest"] = (
            bundle_declaration.get("member_count") == len(names)
            and bundle_declaration.get("ordered_member_names_sha256") == ordered_names_hash
            and bundle_declaration.get("compressed_bytes") == total_compressed
            and bundle_declaration.get("uncompressed_bytes") == total_uncompressed
        )

        def read_member(member: object, maximum: int) -> bytes | None:
            if not safe_zip_member(member, file_only=True) or member not in info_by_name:
                return None
            info = info_by_name[member]
            if info.is_dir() or not 2 <= info.file_size <= maximum or (info.flag_bits & 0x1):
                return None
            try:
                value = archive.read(info)
            except (RuntimeError, NotImplementedError, KeyError, zipfile.BadZipFile):
                return None
            return value if len(value) == info.file_size else None

        def parse_object(value: bytes | None) -> dict[str, Any] | None:
            if value is None:
                return None
            try:
                parsed = json.loads(value.decode("utf-8-sig"))
            except (UnicodeDecodeError, json.JSONDecodeError):
                return None
            return parsed if isinstance(parsed, dict) else None

        collector_bytes = read_member(collector_ref.get("member"), MAX_SOURCE_REPORT_BYTES)
        candidate_bytes = read_member(candidate_ref.get("member"), MAX_SOURCE_REPORT_BYTES)
        observation_schema_bytes = read_member(observation_schema_ref.get("member"), MAX_SOURCE_REPORT_BYTES)
        named_case_manifest_bytes = read_member(named_case_manifest_ref.get("member"), MAX_SOURCE_REPORT_BYTES)
        collector_payload = parse_object(collector_bytes)
        candidate_payload = parse_object(candidate_bytes)
        checks["collector_and_candidate_reports_exist_and_match_manifest_hashes"] = (
            collector_bytes is not None
            and sha256_bytes(collector_bytes) == collector_ref.get("sha256")
            and candidate_bytes is not None
            and sha256_bytes(candidate_bytes) == candidate_ref.get("sha256")
            and observation_schema_bytes is not None
            and sha256_bytes(observation_schema_bytes) == observation_schema_ref.get("sha256")
            and observation_schema_ref.get("sha256") == observation_schema_hash
            and named_case_manifest_bytes is not None
            and sha256_bytes(named_case_manifest_bytes) == named_case_manifest_hash
        )

        candidate_outcomes: dict[str, dict[str, Any]] = {}
        candidate_source_commit: object = None
        if isinstance(candidate_payload, dict):
            raw_outcomes = candidate_payload.get("outcomes")
            if isinstance(raw_outcomes, list):
                candidate_outcomes = {
                    item.get("name"): item
                    for item in raw_outcomes
                    if isinstance(item, dict) and isinstance(item.get("name"), str)
                }
            candidate_source_commit = candidate_payload.get("candidate_build_source_commit")
        checks["candidate_report_is_exact_passing_source_stage_run"] = (
            isinstance(candidate_payload, dict)
            and candidate_payload.get("schema_version") == 3
            and candidate_payload.get("suite_id") == CANDIDATE_SUITE
            and candidate_payload.get("target_release") == TARGET_RELEASE
            and candidate_payload.get("passed") is True
            and candidate_payload.get("source_worktree_clean") is True
            and candidate_payload.get("named_evidence_stage") == "source"
            and candidate_payload.get("named_evidence_verified") is True
            and candidate_source_commit == named_manifest.get("source_commit")
            and set(candidate_outcomes) == {"portable", "installed"}
            and all(candidate_outcomes[name].get("status") == "passed" for name in candidate_outcomes)
            and all(candidate_outcomes[name].get("build_source_commit") == candidate_source_commit for name in candidate_outcomes)
            and all(
                isinstance(candidate_outcomes[name].get("executable_sha256"), str)
                and candidate_outcomes[name]["executable_sha256"].lower() == candidate_digests.get(name)
                for name in candidate_outcomes
            )
        )

        collection_sources = collector_payload.get("sources") if isinstance(collector_payload, dict) and isinstance(collector_payload.get("sources"), dict) else {}
        collection_provenance = collector_payload.get("provenance") if isinstance(collector_payload, dict) and isinstance(collector_payload.get("provenance"), dict) else {}
        collection_summary = collector_payload.get("summary") if isinstance(collector_payload, dict) and isinstance(collector_payload.get("summary"), dict) else {}
        collection_cases_raw = collector_payload.get("cases") if isinstance(collector_payload, dict) else None
        collection_cases = {
            item.get("id"): item
            for item in collection_cases_raw
            if isinstance(item, dict) and isinstance(item.get("id"), str)
        } if isinstance(collection_cases_raw, list) else {}
        expected_ids = {str(entry.get("id", "")) for entry in entries}
        checks["collector_report_is_exact_passing_55_case_collection"] = (
            isinstance(collector_payload, dict)
            and collector_payload.get("schema_version") == 1
            and collector_payload.get("suite_id") == COLLECTOR_SUITE
            and collector_payload.get("target_release") == TARGET_RELEASE
            and collector_payload.get("status") == "passed"
            and collector_payload.get("passed") is True
            and not collector_payload.get("failures")
            and not collector_payload.get("missing_concrete_drivers")
            and collection_summary.get("required") == 55
            and collection_summary.get("collected") == 55
            and collection_summary.get("missing_concrete_driver") == 0
            and collection_summary.get("failed") == 0
            and isinstance(collection_cases_raw, list)
            and len(collection_cases_raw) == len(collection_cases) == 55
            and set(collection_cases) == expected_ids
            and all(item.get("status") == "collected" for item in collection_cases.values())
        )
        checks["collector_report_binds_current_matrix_index_candidate_and_commit"] = (
            collection_sources.get("matrix_sha256") == matrix_hash
            and collection_sources.get("output_index_sha256") == index_hash
            and collection_sources.get("candidate_report_member") == candidate_ref.get("member")
            and collection_sources.get("candidate_report_sha256") == candidate_ref.get("sha256")
            and collection_sources.get("observation_schema_member") == observation_schema_ref.get("member")
            and collection_sources.get("observation_schema_sha256") == observation_schema_hash
            and collection_sources.get("named_case_manifest_member") == named_case_manifest_ref.get("member")
            and collection_sources.get("named_case_manifest_sha256") == named_case_manifest_hash
            and collection_provenance.get("source_commit") == candidate_source_commit
            and collection_provenance.get("candidate_executables") == candidate_digests
        )
        declared_collection_members = collector_payload.get("bundle_members") if isinstance(collector_payload, dict) else None
        required_index_members = {
            member
            for entry in entries
            for member in (
                entry.get("screenshot", {}).get("member") if isinstance(entry.get("screenshot"), dict) else None,
                entry.get("receipt", {}).get("member") if isinstance(entry.get("receipt"), dict) else None,
            )
            if isinstance(member, str)
        }
        checks["collector_declared_members_are_unique_safe_present_and_cover_index"] = (
            isinstance(declared_collection_members, list)
            and len(declared_collection_members) == len(set(declared_collection_members))
            and all(safe_zip_member(member, file_only=True) for member in declared_collection_members)
            and set(declared_collection_members).issubset(set(names))
            and collector_ref.get("member") in declared_collection_members
            and candidate_ref.get("member") in declared_collection_members
            and observation_schema_ref.get("member") in declared_collection_members
            and named_case_manifest_ref.get("member") in declared_collection_members
            and required_index_members.issubset(set(declared_collection_members))
        )
        provenance = {
            "collector_report_member": collector_ref.get("member"),
            "collector_report_sha256": collector_ref.get("sha256"),
            "candidate_report_member": candidate_ref.get("member"),
            "candidate_report_sha256": candidate_ref.get("sha256"),
            "observation_schema_member": observation_schema_ref.get("member"),
            "observation_schema_sha256": observation_schema_ref.get("sha256"),
            "named_case_manifest_member": named_case_manifest_ref.get("member"),
            "named_case_manifest_sha256": named_case_manifest_hash,
            "source_commit": candidate_source_commit,
            "candidate_executables": candidate_digests,
            "bundle_member_count": len(names),
            "bundle_compressed_bytes": total_compressed,
            "bundle_uncompressed_bytes": total_uncompressed,
            "bundle_ordered_member_names_sha256": ordered_names_hash,
        }

        screenshot_members = Counter(
            entry.get("screenshot", {}).get("member")
            for entry in entries if isinstance(entry.get("screenshot"), dict)
        )
        screenshot_hashes = Counter(
            entry.get("screenshot", {}).get("sha256")
            for entry in entries if isinstance(entry.get("screenshot"), dict)
        )
        receipt_members = Counter(
            entry.get("receipt", {}).get("member")
            for entry in entries if isinstance(entry.get("receipt"), dict)
        )
        receipt_hashes = Counter(
            entry.get("receipt", {}).get("sha256")
            for entry in entries if isinstance(entry.get("receipt"), dict)
        )
        source_payload_cache: dict[tuple[str, str], dict[str, Any] | None] = {}
        source_observation_keys: list[tuple[str, str] | None] = []
        trusted_rows = collector_contract.get("trusted_driver_suites")
        trusted_suites = {
            row.get("suite_id"): row.get("schema_version")
            for row in trusted_rows
            if isinstance(row, dict)
        } if isinstance(trusted_rows, list) else {}

        for entry in entries:
            case_id = str(entry.get("id", ""))
            report: dict[str, Any] = {"id": case_id, "status": "failed", "checks": {}}
            screenshot = entry.get("screenshot") if isinstance(entry.get("screenshot"), dict) else {}
            receipt = entry.get("receipt") if isinstance(entry.get("receipt"), dict) else {}
            binding = receipt.get("binding") if isinstance(receipt.get("binding"), dict) else {}
            screenshot_member = screenshot.get("member")
            screenshot_hash = screenshot.get("sha256")
            receipt_member = receipt.get("member")
            receipt_hash = receipt.get("sha256")
            pointer = binding.get("json_pointer")
            report.update(
                {
                    "screenshot_member": screenshot_member,
                    "receipt_member": receipt_member,
                    "binding_pointer": pointer,
                }
            )
            per_case: dict[str, bool] = report["checks"]
            per_case["entry_is_verified"] = entry.get("status") == "verified"
            per_case["case_artifact_members_hashes_are_unique"] = (
                screenshot_members[screenshot_member] == 1
                and screenshot_hashes[screenshot_hash] == 1
                and receipt_members[receipt_member] == 1
                and receipt_hashes[receipt_hash] == 1
            )
            per_case["index_members_are_safe_and_hashed"] = (
                safe_zip_member(screenshot_member, file_only=True)
                and isinstance(screenshot_hash, str)
                and SHA256_RE.fullmatch(screenshot_hash) is not None
                and safe_zip_member(receipt_member, file_only=True)
                and isinstance(receipt_hash, str)
                and SHA256_RE.fullmatch(receipt_hash) is not None
            )
            screenshot_bytes = read_member(screenshot_member, MAX_SCREENSHOT_BYTES)
            receipt_bytes = read_member(receipt_member, MAX_RECEIPT_BYTES)
            per_case["index_members_exist_once_and_match_sha256"] = (
                screenshot_bytes is not None
                and receipt_bytes is not None
                and names.count(screenshot_member) == 1
                and names.count(receipt_member) == 1
                and sha256_bytes(screenshot_bytes) == screenshot_hash
                and sha256_bytes(receipt_bytes) == receipt_hash
            )
            png_ok, png_width, png_height = validate_png_bytes(screenshot_bytes or b"")
            per_case["screenshot_is_structurally_valid_png"] = png_ok
            receipt_payload = parse_object(receipt_bytes)
            per_case["receipt_is_exact_collector_case_receipt"] = (
                isinstance(receipt_payload, dict)
                and receipt_payload.get("schema_version") == 1
                and receipt_payload.get("suite_id") == CASE_RECEIPT_SUITE
                and receipt_payload.get("target_release") == TARGET_RELEASE
                and receipt_payload.get("status") == "passed"
                and receipt_payload.get("case_id") == case_id
                and receipt_payload.get("scope") == entry.get("scope")
                and receipt_payload.get("group") == entry.get("group")
                and receipt_payload.get("case") == entry.get("case")
            )
            binding_value: Any = None
            try:
                binding_value = pointer_value(receipt_payload, pointer) if isinstance(receipt_payload, dict) else None
            except (ValueError, KeyError, TypeError):
                binding_value = None
            per_case["receipt_json_pointer_binds_exact_case_id"] = (
                pointer == "/case_id"
                and binding.get("expected_value") == case_id
                and binding_value == case_id
                and isinstance(receipt_payload, dict)
                and receipt_payload.get("case_binding") == {
                    "json_pointer": "/case_id",
                    "expected_value": case_id,
                }
            )
            expected_op = expected_operation(entry, collector_contract)
            expected_assertion = expected_assertion_id(entry, expected_op)
            receipt_assertion = receipt_payload.get("assertion") if isinstance(receipt_payload, dict) and isinstance(receipt_payload.get("assertion"), dict) else {}
            per_case["receipt_binds_exact_operation_and_assertion"] = (
                isinstance(receipt_payload, dict)
                and receipt_payload.get("operation") == expected_op
                and receipt_assertion.get("id") == expected_assertion
                and receipt_assertion.get("passed") is True
                and "expected" in receipt_assertion
                and receipt_assertion.get("expected") is not None
                and receipt_assertion.get("expected") == receipt_assertion.get("observed")
            )

            receipt_candidate = receipt_payload.get("candidate") if isinstance(receipt_payload, dict) and isinstance(receipt_payload.get("candidate"), dict) else {}
            candidate_name = receipt_candidate.get("name")
            selected_candidate_name = expected_candidate_name(entry, collector_contract)
            candidate_outcome = candidate_outcomes.get(candidate_name) if isinstance(candidate_name, str) else None
            per_case["receipt_binds_exact_candidate_sha_version_and_source_commit"] = (
                isinstance(candidate_outcome, dict)
                and candidate_name == selected_candidate_name
                and isinstance(candidate_outcome.get("executable_sha256"), str)
                and receipt_candidate.get("executable_sha256") == candidate_outcome["executable_sha256"].lower()
                and receipt_candidate.get("product_version") == candidate_outcome.get("product_version")
                and receipt_candidate.get("source_commit") == candidate_source_commit
            )
            receipt_candidate_report = receipt_payload.get("candidate_report") if isinstance(receipt_payload, dict) and isinstance(receipt_payload.get("candidate_report"), dict) else {}
            per_case["receipt_binds_exact_candidate_report"] = (
                receipt_candidate_report.get("member") == candidate_ref.get("member")
                and receipt_candidate_report.get("sha256") == candidate_ref.get("sha256")
                and receipt_candidate_report.get("schema_version") == 3
                and receipt_candidate_report.get("suite_id") == CANDIDATE_SUITE
            )

            receipt_screenshot = receipt_payload.get("screenshot") if isinstance(receipt_payload, dict) and isinstance(receipt_payload.get("screenshot"), dict) else {}
            per_case["receipt_binds_exact_screenshot_bytes_and_dimensions"] = (
                receipt_screenshot.get("member") == screenshot_member
                and receipt_screenshot.get("sha256") == screenshot_hash
                and receipt_screenshot.get("width") == png_width
                and receipt_screenshot.get("height") == png_height
            )

            source_ref = receipt_payload.get("source_report") if isinstance(receipt_payload, dict) and isinstance(receipt_payload.get("source_report"), dict) else {}
            source_member = source_ref.get("member")
            source_hash = source_ref.get("sha256")
            source_pointer = source_ref.get("json_pointer")
            source_key = (source_hash, source_pointer) if isinstance(source_hash, str) and isinstance(source_pointer, str) else None
            source_observation_keys.append(source_key)
            per_case["source_report_ref_is_safe_hashed_and_trusted"] = (
                safe_zip_member(source_member, file_only=True)
                and isinstance(source_hash, str)
                and SHA256_RE.fullmatch(source_hash) is not None
                and source_ref.get("suite_id") in trusted_suites
                and source_ref.get("schema_version") == trusted_suites.get(source_ref.get("suite_id"))
                and isinstance(source_pointer, str)
                and source_pointer.startswith("/named_evidence_observations/")
                and isinstance(source_ref.get("observation_sha256"), str)
                and SHA256_RE.fullmatch(source_ref["observation_sha256"]) is not None
            )
            cache_key = (str(source_member), str(source_hash))
            if cache_key not in source_payload_cache:
                source_bytes = read_member(source_member, MAX_SOURCE_REPORT_BYTES)
                source_payload_cache[cache_key] = (
                    parse_object(source_bytes)
                    if source_bytes is not None and sha256_bytes(source_bytes) == source_hash
                    else None
                )
            source_payload = source_payload_cache[cache_key]
            per_case["source_report_member_exists_and_matches_sha256"] = source_payload is not None
            per_case["source_report_is_exact_passing_trusted_suite"] = (
                isinstance(source_payload, dict)
                and source_payload.get("suite_id") == source_ref.get("suite_id")
                and source_payload.get("schema_version") == source_ref.get("schema_version")
                and driver_report_passed(source_payload)
            )
            per_case["generic_driver_binds_executed_named_case_manifest"] = (
                not isinstance(source_payload, dict)
                or source_payload.get("suite_id") != "quickpls_v255_named_case_driver_v1"
                or (
                    isinstance(source_payload.get("sources"), dict)
                    and source_payload["sources"].get("manifest_sha256") == named_case_manifest_hash
                    and source_payload.get("candidate") == candidate_name
                    and any(
                        isinstance(row, dict) and row.get("id") == case_id and row.get("status") == "passed"
                        for row in (source_payload.get("cases") or [])
                    )
                )
            )
            source_observation: Any = None
            try:
                source_observation = pointer_value(source_payload, source_pointer) if isinstance(source_payload, dict) else None
            except (ValueError, KeyError, TypeError):
                source_observation = None
            source_assertion = source_observation.get("assertion") if isinstance(source_observation, dict) and isinstance(source_observation.get("assertion"), dict) else {}
            source_screenshot = source_observation.get("screenshot") if isinstance(source_observation, dict) and isinstance(source_observation.get("screenshot"), dict) else {}
            per_case["source_pointer_resolves_exact_case_operation_assertion_and_screenshot"] = (
                isinstance(source_observation, dict)
                and source_observation.get("schema_version") == 1
                and source_observation.get("case_id") == case_id
                and source_observation.get("operation") == expected_op
                and source_assertion.get("id") == expected_assertion
                and source_assertion.get("passed") is True
                and source_assertion.get("expected") is not None
                and source_assertion.get("expected") == source_assertion.get("observed")
                and source_assertion.get("expected") == receipt_assertion.get("expected")
                and source_assertion.get("observed") == receipt_assertion.get("observed")
                and source_screenshot.get("sha256") == screenshot_hash
                and sha256_bytes(canonical_json_bytes(source_observation)) == source_ref.get("observation_sha256")
            )
            collection_case = collection_cases.get(case_id)
            per_case["collector_report_case_row_binds_all_artifacts_and_provenance"] = (
                isinstance(collection_case, dict)
                and collection_case.get("status") == "collected"
                and collection_case.get("candidate") == candidate_name
                and collection_case.get("candidate_executable_sha256") == receipt_candidate.get("executable_sha256")
                and collection_case.get("source_report_member") == source_member
                and collection_case.get("source_report_sha256") == source_hash
                and collection_case.get("source_json_pointer") == source_pointer
                and collection_case.get("source_observation_sha256") == source_ref.get("observation_sha256")
                and collection_case.get("screenshot_member") == screenshot_member
                and collection_case.get("screenshot_sha256") == screenshot_hash
                and collection_case.get("receipt_member") == receipt_member
                and collection_case.get("receipt_sha256") == receipt_hash
            )
            report["candidate"] = candidate_name
            report["source_report_member"] = source_member
            report["source_json_pointer"] = source_pointer
            report["source_observation_key"] = list(source_key) if source_key else None
            case_reports.append(report)

        source_key_counts = Counter(key for key in source_observation_keys if key is not None)
        for report in case_reports:
            raw_key = report.get("source_observation_key")
            key = tuple(raw_key) if isinstance(raw_key, list) and len(raw_key) == 2 else None
            report["checks"]["source_report_and_pointer_are_unique_to_case"] = (
                key is not None and source_key_counts[key] == 1
            )
            if all(report["checks"].values()):
                report["status"] = "passed"
            else:
                failures.extend(
                    f"{report['id']}:{name}"
                    for name, passed in report["checks"].items()
                    if not passed
                )

    failures.extend(name for name, passed in checks.items() if not passed)
    return case_reports, list(dict.fromkeys(failures)), checks, bundle_hash, provenance


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify QuickPLS 2.55 named cross/specialized evidence.")
    parser.add_argument("--stage", choices=("source", "publication"), default="source")
    parser.add_argument("--matrix", type=Path, default=DEFAULT_MATRIX)
    parser.add_argument("--index", type=Path, default=DEFAULT_INDEX)
    parser.add_argument("--bundle-manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--observation-schema", type=Path, default=DEFAULT_OBSERVATION_SCHEMA)
    parser.add_argument("--named-case-manifest", type=Path, default=DEFAULT_NAMED_CASE_MANIFEST)
    parser.add_argument("--evidence-bundle", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    matrix_path = args.matrix.resolve()
    index_path = args.index.resolve()
    manifest_path = args.bundle_manifest.resolve()
    observation_schema_path = args.observation_schema.resolve()
    named_case_manifest_path = args.named_case_manifest.resolve()
    output_path = args.output.resolve()
    failures: list[str] = []
    checks: dict[str, bool] = {}
    case_reports: list[dict[str, Any]] = []
    bundle_hash: str | None = None
    publication_provenance: dict[str, Any] = {}
    try:
        matrix = load_json(matrix_path)
        index = load_json(index_path)
        manifest = load_json(manifest_path)
        observation_schema = load_json(observation_schema_path)
        named_case_manifest = load_json(named_case_manifest_path)
        if not (
            observation_schema.get("$id") == "https://quickpls.local/schemas/v255-named-evidence-observations-v1.json"
            and observation_schema.get("type") == "array"
        ):
            raise ValueError("named-evidence observation schema has an unsupported identity")
        _, entries, shape_failures, shape_checks = validate_index_shape(
            matrix, index, args.stage == "publication"
        )
        checks.update(shape_checks)
        failures.extend(shape_failures)
        manifest_partition = list(named_case_manifest.get("supplied_by_fixed_drivers", [])) + [row.get("id") for row in named_case_manifest.get("cases", []) if isinstance(row, dict)] + [row.get("id") for row in named_case_manifest.get("pending_cases", []) if isinstance(row, dict)]
        expected_ids = [entry.get("id") for entry in entries]
        checks["named_case_manifest_is_exact_complete_partition"] = (
            named_case_manifest.get("schema_version") == 1
            and named_case_manifest.get("suite_id") == "quickpls_v255_named_case_manifest_v1"
            and named_case_manifest.get("target_release") == TARGET_RELEASE
            and named_case_manifest.get("status") == "ready"
            and named_case_manifest.get("coverage_status") == "complete"
            and named_case_manifest.get("pending_cases") == []
            and len(manifest_partition) == len(set(manifest_partition)) == 55
            and set(manifest_partition) == set(expected_ids)
        )
        if not checks["named_case_manifest_is_exact_complete_partition"]:
            failures.append("named_case_manifest_is_exact_complete_partition")
        checks["named_case_manifest_has_zero_unimplemented_source_routes"] = (
            isinstance(named_case_manifest.get("pending_cases"), list)
            and len(named_case_manifest["pending_cases"]) == 0
        )
        if not checks["named_case_manifest_has_zero_unimplemented_source_routes"]:
            failures.append("named_case_manifest_has_zero_unimplemented_source_routes")
        if args.stage == "publication":
            if args.evidence_bundle is None or not args.evidence_bundle.resolve().is_file():
                checks["publication_evidence_bundle_is_supplied"] = False
                failures.append("publication_evidence_bundle_is_supplied")
            else:
                checks["publication_evidence_bundle_is_supplied"] = True
                collector_contract = index.get("collector_contract")
                if not isinstance(collector_contract, dict):
                    raise ValueError("named-evidence index lacks collector_contract")
                publication_cases, publication_failures, publication_checks, bundle_hash, publication_provenance = verify_publication_entries(
                    entries,
                    args.evidence_bundle.resolve(),
                    manifest,
                    sha256_path(matrix_path),
                    sha256_path(index_path),
                    sha256_path(observation_schema_path),
                    sha256_path(named_case_manifest_path),
                    collector_contract,
                )
                case_reports.extend(publication_cases)
                checks.update(publication_checks)
                failures.extend(publication_failures)
        else:
            checks["source_stage_allows_pending_collection"] = all(
                entry.get("status") in {"pending", "verified"} for entry in entries
            )
            if not checks["source_stage_allows_pending_collection"]:
                failures.append("source_stage_allows_pending_collection")

        failures = list(dict.fromkeys(failures))
        payload = {
            "schema_version": 1,
            "suite_id": "quickpls_v255_named_evidence_verifier_v1",
            "target_release": TARGET_RELEASE,
            "stage": args.stage,
            "passed": not failures,
            "sources": {
                "matrix": str(matrix_path),
                "matrix_sha256": sha256_path(matrix_path),
                "index": str(index_path),
                "index_sha256": sha256_path(index_path),
                "bundle_manifest": str(manifest_path),
                "bundle_manifest_sha256": sha256_path(manifest_path),
                "observation_schema": str(observation_schema_path),
                "observation_schema_sha256": sha256_path(observation_schema_path),
                "named_case_manifest": str(named_case_manifest_path),
                "named_case_manifest_sha256": sha256_path(named_case_manifest_path),
                "evidence_bundle": str(args.evidence_bundle.resolve()) if args.evidence_bundle else None,
                "evidence_bundle_sha256": bundle_hash,
                "publication_provenance": publication_provenance or None,
            },
            "summary": {
                "required": 55,
                "cross_method_required": 29,
                "specialized_result_required": 26,
                "verified": sum(case.get("status") == "passed" for case in case_reports),
                "pending": sum(entry.get("status") == "pending" for entry in entries),
                "failed": len(failures),
            },
            "checks": checks,
            "cases": case_reports,
            "failures": failures,
        }
    except (OSError, ValueError, RuntimeError, zipfile.BadZipFile, json.JSONDecodeError) as error:
        payload = {
            "schema_version": 1,
            "suite_id": "quickpls_v255_named_evidence_verifier_v1",
            "target_release": TARGET_RELEASE,
            "stage": args.stage,
            "passed": False,
            "sources": {
                "matrix": str(matrix_path),
                "index": str(index_path),
                "bundle_manifest": str(manifest_path),
                "evidence_bundle": str(args.evidence_bundle.resolve()) if args.evidence_bundle else None,
            },
            "checks": checks,
            "cases": case_reports,
            "failures": [f"verifier_error:{error}"],
        }

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(payload, indent=2))
    return 0 if payload["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
