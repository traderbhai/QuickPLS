from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import shutil
import struct
import sys
import zlib
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MATRIX = ROOT / "validation" / "v255_method_evidence_matrix.json"
DEFAULT_INDEX = ROOT / "validation" / "v255_named_evidence_index.json"
DEFAULT_MANIFEST = ROOT / "validation" / "v255_evidence_bundle_manifest.json"
DEFAULT_OBSERVATION_SCHEMA = ROOT / "validation" / "v255_named_evidence_observation.schema.json"
DEFAULT_NAMED_CASE_MANIFEST = ROOT / "validation" / "v255_named_case_manifest.json"
TARGET_RELEASE = "2.55.0"
COLLECTOR_SUITE = "quickpls_v255_named_evidence_collector_v1"
CASE_RECEIPT_SUITE = "quickpls_v255_named_evidence_case_receipt_v1"
CANDIDATE_SUITE = "quickpls_v255_installed_portable_smoke_v3"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
SAFE_FILE_RE = re.compile(r"[^a-z0-9]+")
MAX_JSON_BYTES = 32 * 1024 * 1024
MAX_SCREENSHOT_BYTES = 64 * 1024 * 1024


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        raise ValueError(f"JSON file is missing: {path}")
    if path.stat().st_size > MAX_JSON_BYTES:
        raise ValueError(f"JSON file exceeds {MAX_JSON_BYTES} bytes: {path}")
    value = json.loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(value, dict):
        raise ValueError(f"JSON root must be an object: {path}")
    return value


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def pretty_json_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_path(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def inside(parent: Path, candidate: Path) -> bool:
    try:
        candidate.resolve().relative_to(parent.resolve())
        return True
    except ValueError:
        return False


def safe_member(member: object) -> bool:
    if not isinstance(member, str) or not member or "\\" in member or "\x00" in member:
        return False
    if member.startswith("/") or ":" in member or member.endswith("/"):
        return False
    path = PurePosixPath(member)
    return (
        path.as_posix() == member
        and not path.is_absolute()
        and all(part not in {"", ".", ".."} for part in path.parts)
    )


def pointer_value(payload: Any, pointer: str) -> Any:
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        raise ValueError(f"Invalid non-root JSON pointer: {pointer!r}")
    current = payload
    for encoded in pointer[1:].split("/"):
        if re.search(r"~(?:[^01]|$)", encoded):
            raise ValueError(f"Invalid JSON pointer escape: {pointer}")
        token = encoded.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict) and token in current:
            current = current[token]
        elif isinstance(current, list) and token.isdigit() and int(token) < len(current):
            current = current[int(token)]
        else:
            raise KeyError(pointer)
    return current


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
        declarations = {
            item.get("case"): item
            for item in evidence if isinstance(item, dict) and isinstance(item.get("case"), str)
        } if isinstance(evidence, list) else {}
        if set(declarations) != set(required) or len(declarations) != len(required):
            raise ValueError(f"matrix declarations do not exactly cover cross-method group: {group}")
        if any(item.get("status") not in {"ready", "post_candidate"} for item in declarations.values()):
            raise ValueError(f"matrix declaration status was mutated for cross-method group: {group}")
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
    declarations = {
        item.get("case"): item
        for item in evidence if isinstance(item, dict) and isinstance(item.get("case"), str)
    } if isinstance(evidence, list) else {}
    if set(declarations) != set(required) or len(declarations) != len(required):
        raise ValueError("matrix declarations do not exactly cover specialized cases")
    if any(item.get("status") not in {"ready", "post_candidate"} for item in declarations.values()):
        raise ValueError("matrix declaration status was mutated for specialized cases")
    rows.extend(
        {
            "id": f"specialized_result:{case}",
            "scope": "specialized_result",
            "group": "specialized_result_evidence",
            "case": case,
        }
        for case in required
    )
    if len(rows) != 55:
        raise ValueError(f"expected exactly 55 frozen named cases, found {len(rows)}")
    return rows


def expected_operation(row: dict[str, str], contract: dict[str, Any]) -> str:
    operations = contract.get("operation_by_group")
    if not isinstance(operations, dict):
        raise ValueError("collector contract operation_by_group must be an object")
    operation = operations.get(row["group"])
    if not isinstance(operation, str) or not operation:
        raise ValueError(f"collector contract lacks an operation for {row['group']}")
    return operation


def expected_assertion_id(row: dict[str, str], operation: str) -> str:
    return f"{operation}:{row['id']}"


def expected_candidate_name(row: dict[str, str], contract: dict[str, Any]) -> str:
    selection = contract.get("candidate_selection")
    if not isinstance(selection, dict) or selection.get("default") not in {"portable", "installed"}:
        raise ValueError("collector contract candidate_selection is invalid")
    overrides = selection.get("overrides")
    if not isinstance(overrides, dict):
        raise ValueError("collector contract candidate_selection overrides must be an object")
    selected = overrides.get(row["id"], selection["default"])
    if selected not in {"portable", "installed"}:
        raise ValueError(f"collector contract candidate selection is invalid for {row['id']}")
    return selected


def validate_png(path: Path) -> tuple[int, int]:
    size = path.stat().st_size
    if size < 45 or size > MAX_SCREENSHOT_BYTES:
        raise ValueError(f"PNG size is outside the allowed range: {path}")
    data = path.read_bytes()
    if not data.startswith(PNG_SIGNATURE):
        raise ValueError(f"screenshot is not a PNG: {path}")
    offset = len(PNG_SIGNATURE)
    width = 0
    height = 0
    saw_iend = False
    chunk_index = 0
    while offset < len(data):
        if offset + 12 > len(data):
            raise ValueError(f"PNG has a truncated chunk header: {path}")
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        chunk_type = data[offset + 4 : offset + 8]
        end = offset + 12 + length
        if end > len(data):
            raise ValueError(f"PNG has a truncated chunk payload: {path}")
        chunk_data = data[offset + 8 : offset + 8 + length]
        expected_crc = struct.unpack(">I", data[offset + 8 + length : end])[0]
        if zlib.crc32(chunk_type + chunk_data) & 0xFFFFFFFF != expected_crc:
            raise ValueError(f"PNG chunk CRC is invalid: {path}")
        if chunk_index == 0:
            if chunk_type != b"IHDR" or length != 13:
                raise ValueError(f"PNG does not begin with a valid IHDR chunk: {path}")
            width, height = struct.unpack(">II", chunk_data[:8])
            if width <= 0 or height <= 0:
                raise ValueError(f"PNG has invalid dimensions: {path}")
        if chunk_type == b"IEND":
            if length != 0 or end != len(data):
                raise ValueError(f"PNG has an invalid IEND or trailing bytes: {path}")
            saw_iend = True
            break
        offset = end
        chunk_index += 1
    if not saw_iend:
        raise ValueError(f"PNG has no IEND chunk: {path}")
    return width, height


def resolve_bound_file(base: Path, raw_path: object, evidence_root: Path, label: str) -> Path:
    if not isinstance(raw_path, str) or not raw_path.strip():
        raise ValueError(f"{label} path is missing")
    value = Path(raw_path)
    candidate = value.resolve() if value.is_absolute() else (base / value).resolve()
    if not inside(evidence_root, candidate):
        raise ValueError(f"{label} escapes the candidate evidence root: {candidate}")
    if not candidate.is_file():
        raise ValueError(f"{label} is missing: {candidate}")
    return candidate


def trusted_suite_versions(contract: dict[str, Any]) -> dict[str, int]:
    rows = contract.get("trusted_driver_suites")
    if not isinstance(rows, list):
        raise ValueError("collector contract trusted_driver_suites must be an array")
    versions: dict[str, int] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise ValueError("trusted driver suite declaration must be an object")
        suite = row.get("suite_id")
        version = row.get("schema_version")
        if not isinstance(suite, str) or not suite or not isinstance(version, int):
            raise ValueError("trusted driver suite declaration is incomplete")
        if suite in versions:
            raise ValueError(f"trusted driver suite is duplicated: {suite}")
        versions[suite] = version
    return versions


def validate_candidate_report(payload: dict[str, Any]) -> tuple[str, dict[str, dict[str, Any]]]:
    if not (
        payload.get("schema_version") == 3
        and payload.get("suite_id") == CANDIDATE_SUITE
        and payload.get("target_release") == TARGET_RELEASE
        and payload.get("passed") is True
        and payload.get("source_worktree_clean") is True
        and payload.get("named_evidence_stage") == "source"
        and payload.get("named_evidence_verified") is True
    ):
        raise ValueError("candidate report is not a passing source-stage QuickPLS 2.55 installed/portable smoke")
    source_commit = payload.get("candidate_build_source_commit")
    if not isinstance(source_commit, str) or COMMIT_RE.fullmatch(source_commit) is None:
        raise ValueError("candidate report source_commit is not an exact lowercase Git commit")
    outcomes = payload.get("outcomes")
    if not isinstance(outcomes, list) or len(outcomes) != 2:
        raise ValueError("candidate report must contain exactly portable and installed outcomes")
    by_name: dict[str, dict[str, Any]] = {}
    for outcome in outcomes:
        if not isinstance(outcome, dict):
            raise ValueError("candidate outcome is not an object")
        name = outcome.get("name")
        digest = outcome.get("executable_sha256")
        version = outcome.get("product_version")
        if (
            name not in {"portable", "installed"}
            or name in by_name
            or outcome.get("status") != "passed"
            or not isinstance(digest, str)
            or re.fullmatch(r"[0-9a-fA-F]{64}", digest) is None
            or not isinstance(version, str)
            or not version.startswith(TARGET_RELEASE)
            or outcome.get("build_source_commit") != source_commit
        ):
            raise ValueError(f"candidate outcome is incomplete or invalid: {name!r}")
        by_name[name] = {**outcome, "executable_sha256": digest.lower()}
    if set(by_name) != {"portable", "installed"}:
        raise ValueError("candidate report lacks exact portable and installed outcomes")
    return source_commit, by_name


def nested(value: dict[str, Any], *keys: str) -> Any:
    current: Any = value
    for key in keys:
        if not isinstance(current, dict):
            return None
        current = current.get(key)
    return current


def report_bindings(outcome: dict[str, Any]) -> list[tuple[object, object, str]]:
    bindings: list[tuple[object, object, str]] = [
        (outcome.get("lifecycle"), outcome.get("lifecycle_sha256"), "lifecycle"),
        (outcome.get("evidence"), outcome.get("evidence_sha256"), "method_evidence"),
        (
            nested(outcome, "frozen_archive_collection", "aggregate_receipt"),
            nested(outcome, "frozen_archive_collection", "aggregate_receipt_sha256"),
            "frozen_archive_reopen",
        ),
        (
            nested(outcome, "posthoc_collection", "execute_receipt"),
            nested(outcome, "posthoc_collection", "execute_receipt_sha256"),
            "posthoc_execute",
        ),
        (
            nested(outcome, "posthoc_collection", "reopen_receipt"),
            nested(outcome, "posthoc_collection", "reopen_receipt_sha256"),
            "posthoc_reopen",
        ),
    ]
    additional = outcome.get("named_evidence_driver_reports", [])
    if additional is None:
        additional = []
    if not isinstance(additional, list):
        raise ValueError("candidate named_evidence_driver_reports must be an array")
    for index, row in enumerate(additional):
        if not isinstance(row, dict):
            raise ValueError("candidate named-evidence driver binding must be an object")
        bindings.append((row.get("path"), row.get("sha256"), f"named_evidence_driver_{index}"))
    return bindings


def driver_report_passed(payload: dict[str, Any]) -> bool:
    return (
        payload.get("target_release") == TARGET_RELEASE
        and (payload.get("passed") is True or payload.get("status") in {"passed", "verified"})
    )


def discover_driver_reports(
    candidate_report_path: Path,
    outcomes: dict[str, dict[str, Any]],
    trusted_versions: dict[str, int],
) -> tuple[list[dict[str, Any]], list[str]]:
    records: list[dict[str, Any]] = []
    failures: list[str] = []
    seen: set[tuple[str, str]] = set()
    evidence_root = candidate_report_path.parent.resolve()
    for candidate_name, outcome in outcomes.items():
        for raw_path, expected_hash, role in report_bindings(outcome):
            if raw_path is None and expected_hash is None:
                continue
            try:
                if not isinstance(expected_hash, str) or SHA256_RE.fullmatch(expected_hash) is None:
                    raise ValueError(f"{role} has no lowercase SHA-256 binding")
                report_path = resolve_bound_file(
                    candidate_report_path.parent,
                    raw_path,
                    evidence_root,
                    f"{candidate_name} {role} report",
                )
                actual_hash = sha256_path(report_path)
                if actual_hash != expected_hash:
                    raise ValueError(f"{role} SHA-256 does not match candidate report")
                key = (str(report_path).casefold(), actual_hash)
                if key in seen:
                    continue
                payload = load_json(report_path)
                suite_id = payload.get("suite_id")
                schema_version = payload.get("schema_version")
                if suite_id not in trusted_versions or schema_version != trusted_versions.get(suite_id):
                    raise ValueError(
                        f"{role} suite/schema is not trusted: {suite_id!r}/{schema_version!r}"
                    )
                if not driver_report_passed(payload):
                    raise ValueError(f"{role} is not a passing 2.55 driver report")
                seen.add(key)
                records.append(
                    {
                        "candidate_name": candidate_name,
                        "candidate_sha256": outcome["executable_sha256"],
                        "candidate_product_version": outcome["product_version"],
                        "role": role,
                        "path": report_path,
                        "sha256": actual_hash,
                        "suite_id": suite_id,
                        "schema_version": schema_version,
                        "payload": payload,
                    }
                )
            except (OSError, ValueError, json.JSONDecodeError) as error:
                failures.append(f"driver_report_invalid:{candidate_name}:{role}:{error}")
    return records, failures


def observations_by_case(
    reports: list[dict[str, Any]], observation_field: str
) -> tuple[dict[str, list[dict[str, Any]]], list[str]]:
    by_case: dict[str, list[dict[str, Any]]] = {}
    failures: list[str] = []
    for report in reports:
        observations = report["payload"].get(observation_field, [])
        if observations is None:
            observations = []
        if not isinstance(observations, list):
            failures.append(
                f"driver_observations_not_array:{report['candidate_name']}:{report['role']}"
            )
            continue
        for index, observation in enumerate(observations):
            pointer = f"/{observation_field}/{index}"
            if not isinstance(observation, dict):
                failures.append(
                    f"driver_observation_not_object:{report['candidate_name']}:{report['role']}:{pointer}"
                )
                continue
            case_id = observation.get("case_id")
            if not isinstance(case_id, str) or not case_id:
                failures.append(
                    f"driver_observation_missing_case_id:{report['candidate_name']}:{report['role']}:{pointer}"
                )
                continue
            by_case.setdefault(case_id, []).append(
                {"report": report, "pointer": pointer, "observation": observation}
            )
    return by_case, failures


def file_slug(case_id: str) -> str:
    slug = SAFE_FILE_RE.sub("-", case_id.casefold()).strip("-")
    return slug[:96] or "case"


def write_new(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as stream:
        stream.write(data)


def copy_new(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        raise ValueError(f"refusing to overwrite staged artifact: {destination}")
    shutil.copyfile(source, destination)


def stage_member(staging: Path, member: str) -> Path:
    if not safe_member(member):
        raise ValueError(f"unsafe staging member: {member}")
    destination = (staging / Path(*PurePosixPath(member).parts)).resolve()
    if not inside(staging, destination):
        raise ValueError(f"staging member escapes output directory: {member}")
    return destination


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Collect QuickPLS 2.55 named evidence only from case-specific observations "
            "inside driver reports already SHA-256-bound by a current source-stage candidate report."
        )
    )
    parser.add_argument("--candidate-report", required=True, type=Path)
    parser.add_argument("--staging-dir", required=True, type=Path)
    parser.add_argument("--matrix", type=Path, default=DEFAULT_MATRIX)
    parser.add_argument("--index", type=Path, default=DEFAULT_INDEX)
    parser.add_argument("--bundle-manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--observation-schema", type=Path, default=DEFAULT_OBSERVATION_SCHEMA)
    parser.add_argument("--named-case-manifest", type=Path, default=DEFAULT_NAMED_CASE_MANIFEST)
    parser.add_argument("--output-index", type=Path)
    parser.add_argument("--output-report", type=Path)
    args = parser.parse_args()

    candidate_report_path = args.candidate_report.resolve()
    staging = args.staging_dir.resolve()
    matrix_path = args.matrix.resolve()
    index_path = args.index.resolve()
    manifest_path = args.bundle_manifest.resolve()
    observation_schema_path = args.observation_schema.resolve()
    named_case_manifest_path = args.named_case_manifest.resolve()
    output_index = (args.output_index or (staging / "v255_named_evidence_index.collected.json")).resolve()
    output_report = (
        args.output_report
        or (staging / "named-evidence" / "provenance" / "v255-named-evidence-collection.json")
    ).resolve()
    collection_member = "named-evidence/provenance/v255-named-evidence-collection.json"
    candidate_member = "named-evidence/provenance/v255-installed-portable-smoke-source.json"
    observation_schema_member = "named-evidence/provenance/v255-named-evidence-observation.schema.json"
    named_case_manifest_member = "named-evidence/provenance/v255-named-case-manifest.json"
    failures: list[str] = []
    case_reports: list[dict[str, Any]] = []
    missing_cases: list[str] = []

    try:
        if staging.exists():
            raise ValueError(f"refusing to reuse or overwrite staging directory: {staging}")
        staging.mkdir(parents=True, exist_ok=False)
        if not inside(staging, output_index) or not inside(staging, output_report):
            raise ValueError("--output-index and --output-report must remain inside the new staging directory")

        matrix = load_json(matrix_path)
        source_index = load_json(index_path)
        manifest = load_json(manifest_path)
        observation_schema = load_json(observation_schema_path)
        named_case_manifest = load_json(named_case_manifest_path)
        if not (
            named_case_manifest.get("schema_version") == 1
            and named_case_manifest.get("suite_id") == "quickpls_v255_named_case_manifest_v1"
            and named_case_manifest.get("target_release") == TARGET_RELEASE
            and named_case_manifest.get("status") == "ready"
            and named_case_manifest.get("coverage_status") == "complete"
            and named_case_manifest.get("pending_cases") == []
        ):
            raise ValueError("named-case manifest has an unsupported identity or collection state")
        named_case_manifest_hash = sha256_path(named_case_manifest_path)
        if not (
            observation_schema.get("$id") == "https://quickpls.local/schemas/v255-named-evidence-observations-v1.json"
            and observation_schema.get("type") == "array"
        ):
            raise ValueError("named-evidence observation schema has an unsupported identity")
        expected = expected_cases(matrix)
        expected_by_id = {row["id"]: row for row in expected}
        entries = source_index.get("entries")
        if not isinstance(entries, list) or len(entries) != 55:
            raise ValueError("named-evidence source index must contain exactly 55 entries")
        index_by_id = {
            entry.get("id"): entry for entry in entries if isinstance(entry, dict)
        }
        if set(index_by_id) != set(expected_by_id) or len(index_by_id) != 55:
            raise ValueError("named-evidence source index does not exactly match the frozen matrix cases")
        contract = source_index.get("collector_contract")
        if not isinstance(contract, dict):
            raise ValueError("named-evidence source index lacks collector_contract")
        if not (
            contract.get("schema_version") == 1
            and contract.get("suite_id") == COLLECTOR_SUITE
            and contract.get("case_receipt_suite_id") == CASE_RECEIPT_SUITE
            and contract.get("candidate_report_suite_id") == CANDIDATE_SUITE
        ):
            raise ValueError("named-evidence collector contract has an unsupported identity")
        observation_field = contract.get("driver_observation_field")
        if not isinstance(observation_field, str) or observation_field != "named_evidence_observations":
            raise ValueError("collector contract driver_observation_field is unsupported")
        trusted_versions = trusted_suite_versions(contract)

        candidate_payload = load_json(candidate_report_path)
        source_commit, outcomes = validate_candidate_report(candidate_payload)
        candidate_report_hash = sha256_path(candidate_report_path)
        candidate_destination = stage_member(staging, candidate_member)
        copy_new(candidate_report_path, candidate_destination)
        observation_schema_hash = sha256_path(observation_schema_path)
        copy_new(observation_schema_path, stage_member(staging, observation_schema_member))
        copy_new(named_case_manifest_path, stage_member(staging, named_case_manifest_member))

        driver_reports, driver_failures = discover_driver_reports(
            candidate_report_path, outcomes, trusted_versions
        )
        failures.extend(driver_failures)
        for report in driver_reports:
            if report["suite_id"] == "quickpls_v255_named_case_driver_v1" and report["payload"].get("sources", {}).get("manifest_sha256") != named_case_manifest_hash:
                failures.append(f"named_case_driver_manifest_hash_mismatch:{report['candidate_name']}:{report['sha256']}")
        observations, observation_failures = observations_by_case(
            driver_reports, observation_field
        )
        failures.extend(observation_failures)
        unknown_cases = sorted(set(observations) - set(expected_by_id))
        failures.extend(f"unknown_named_case_observation:{case_id}" for case_id in unknown_cases)

        collected_index = copy.deepcopy(source_index)
        collected_by_id = {entry["id"]: entry for entry in collected_index["entries"]}
        used_screenshot_hashes: dict[str, str] = {}
        used_screenshot_members: set[str] = set()
        used_observations: set[tuple[str, str]] = set()
        used_receipt_hashes: dict[str, str] = {}
        used_receipt_members: set[str] = set()
        staged_source_reports: dict[str, str] = {}

        for ordinal, row in enumerate(expected, start=1):
            case_id = row["id"]
            expected_op = expected_operation(row, contract)
            expected_assertion = expected_assertion_id(row, expected_op)
            expected_candidate = expected_candidate_name(row, contract)
            candidates = [
                item for item in observations.get(case_id, [])
                if item["report"]["candidate_name"] == expected_candidate
            ]
            case_report: dict[str, Any] = {
                "id": case_id,
                "scope": row["scope"],
                "group": row["group"],
                "case": row["case"],
                "expected_operation": expected_op,
                "expected_assertion_id": expected_assertion,
                "expected_candidate": expected_candidate,
                "status": "missing_concrete_driver",
            }
            if not candidates:
                missing_cases.append(case_id)
                case_reports.append(case_report)
                continue
            if len(candidates) != 1:
                case_report["status"] = "failed"
                case_report["failure"] = f"expected one exact driver observation, found {len(candidates)}"
                failures.append(f"ambiguous_named_case_observation:{case_id}:{len(candidates)}")
                case_reports.append(case_report)
                continue

            selected = candidates[0]
            report = selected["report"]
            pointer = selected["pointer"]
            observation = selected["observation"]
            try:
                assertion = observation.get("assertion")
                screenshot = observation.get("screenshot")
                if observation.get("schema_version") != 1:
                    raise ValueError("observation schema_version must be 1")
                if observation.get("operation") != expected_op:
                    raise ValueError("operation does not match the frozen case contract")
                if not isinstance(assertion, dict):
                    raise ValueError("assertion must be an object")
                if assertion.get("id") != expected_assertion:
                    raise ValueError("assertion id does not match the frozen case contract")
                if assertion.get("passed") is not True:
                    raise ValueError("assertion is not passed")
                if "expected" not in assertion or "observed" not in assertion:
                    raise ValueError("assertion lacks expected and observed values")
                if assertion.get("expected") is None or assertion.get("expected") != assertion.get("observed"):
                    raise ValueError("assertion expected/observed values do not exactly match")
                if not isinstance(screenshot, dict):
                    raise ValueError("observation screenshot must be an object")
                declared_screenshot_hash = screenshot.get("sha256")
                if not isinstance(declared_screenshot_hash, str) or SHA256_RE.fullmatch(declared_screenshot_hash) is None:
                    raise ValueError("observation screenshot lacks a lowercase SHA-256")
                screenshot_path = resolve_bound_file(
                    report["path"].parent,
                    screenshot.get("path"),
                    candidate_report_path.parent.resolve(),
                    f"{case_id} screenshot",
                )
                width, height = validate_png(screenshot_path)
                actual_screenshot_hash = sha256_path(screenshot_path)
                if actual_screenshot_hash != declared_screenshot_hash:
                    raise ValueError("observation screenshot SHA-256 does not match current bytes")
                if actual_screenshot_hash in used_screenshot_hashes:
                    raise ValueError(
                        f"screenshot bytes were already used by {used_screenshot_hashes[actual_screenshot_hash]}"
                    )
                source_observation_key = (report["sha256"], pointer)
                if source_observation_key in used_observations:
                    raise ValueError("source report observation pointer was already used by another case")

                source_member = staged_source_reports.get(report["sha256"])
                if source_member is None:
                    source_member = f"named-evidence/source-reports/{report['sha256']}.json"
                    copy_new(report["path"], stage_member(staging, source_member))
                    staged_source_reports[report["sha256"]] = source_member

                slug = file_slug(case_id)
                screenshot_member = f"named-evidence/screenshots/{ordinal:02d}-{slug}.png"
                receipt_member = f"named-evidence/receipts/{ordinal:02d}-{slug}.json"
                if screenshot_member in used_screenshot_members or receipt_member in used_receipt_members:
                    raise ValueError("derived bundle member collides with another case")
                copy_new(screenshot_path, stage_member(staging, screenshot_member))
                observation_hash = sha256_bytes(canonical_json_bytes(observation))
                case_receipt = {
                    "schema_version": 1,
                    "suite_id": CASE_RECEIPT_SUITE,
                    "target_release": TARGET_RELEASE,
                    "status": "passed",
                    "case_id": case_id,
                    "scope": row["scope"],
                    "group": row["group"],
                    "case": row["case"],
                    "operation": expected_op,
                    "assertion": {
                        "id": expected_assertion,
                        "passed": True,
                        "expected": assertion["expected"],
                        "observed": assertion["observed"],
                    },
                    "candidate": {
                        "name": report["candidate_name"],
                        "executable_sha256": report["candidate_sha256"],
                        "product_version": report["candidate_product_version"],
                        "source_commit": source_commit,
                    },
                    "candidate_report": {
                        "member": candidate_member,
                        "sha256": candidate_report_hash,
                        "schema_version": 3,
                        "suite_id": CANDIDATE_SUITE,
                    },
                    "source_report": {
                        "member": source_member,
                        "sha256": report["sha256"],
                        "schema_version": report["schema_version"],
                        "suite_id": report["suite_id"],
                        "json_pointer": pointer,
                        "observation_sha256": observation_hash,
                    },
                    "screenshot": {
                        "member": screenshot_member,
                        "sha256": actual_screenshot_hash,
                        "width": width,
                        "height": height,
                    },
                    "case_binding": {
                        "json_pointer": "/case_id",
                        "expected_value": case_id,
                    },
                }
                receipt_bytes = pretty_json_bytes(case_receipt)
                receipt_hash = sha256_bytes(receipt_bytes)
                if receipt_hash in used_receipt_hashes:
                    raise ValueError(
                        f"receipt bytes were already used by {used_receipt_hashes[receipt_hash]}"
                    )
                write_new(stage_member(staging, receipt_member), receipt_bytes)

                used_screenshot_hashes[actual_screenshot_hash] = case_id
                used_screenshot_members.add(screenshot_member)
                used_observations.add(source_observation_key)
                used_receipt_hashes[receipt_hash] = case_id
                used_receipt_members.add(receipt_member)
                index_entry = collected_by_id[case_id]
                index_entry["status"] = "verified"
                index_entry["screenshot"] = {
                    "member": screenshot_member,
                    "sha256": actual_screenshot_hash,
                }
                index_entry["receipt"] = {
                    "member": receipt_member,
                    "sha256": receipt_hash,
                    "binding": {
                        "json_pointer": "/case_id",
                        "expected_value": case_id,
                    },
                }
                case_report.update(
                    {
                        "status": "collected",
                        "candidate": report["candidate_name"],
                        "candidate_executable_sha256": report["candidate_sha256"],
                        "source_report_member": source_member,
                        "source_report_sha256": report["sha256"],
                        "source_json_pointer": pointer,
                        "source_observation_sha256": observation_hash,
                        "screenshot_member": screenshot_member,
                        "screenshot_sha256": actual_screenshot_hash,
                        "receipt_member": receipt_member,
                        "receipt_sha256": receipt_hash,
                    }
                )
            except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
                case_report["status"] = "failed"
                case_report["failure"] = str(error)
                failures.append(f"named_case_collection_failed:{case_id}:{error}")
            case_reports.append(case_report)

        collected_count = sum(case.get("status") == "collected" for case in case_reports)
        collected_index["status"] = "verified" if collected_count == 55 and not failures else "pending_collection"
        output_index_bytes = pretty_json_bytes(collected_index)
        write_new(output_index, output_index_bytes)
        output_index_hash = sha256_bytes(output_index_bytes)

        bundle_members = sorted(
            [candidate_member, collection_member, observation_schema_member, named_case_manifest_member]
            + list(staged_source_reports.values())
            + [case["screenshot_member"] for case in case_reports if case.get("status") == "collected"]
            + [case["receipt_member"] for case in case_reports if case.get("status") == "collected"]
        )
        collection_passed = collected_count == 55 and not failures and not missing_cases
        collection_report = {
            "schema_version": 1,
            "suite_id": COLLECTOR_SUITE,
            "target_release": TARGET_RELEASE,
            "status": "passed" if collection_passed else "blocked",
            "passed": collection_passed,
            "sources": {
                "matrix": str(matrix_path),
                "matrix_sha256": sha256_path(matrix_path),
                "input_index": str(index_path),
                "input_index_sha256": sha256_path(index_path),
                "output_index": str(output_index),
                "output_index_sha256": output_index_hash,
                "bundle_manifest": str(manifest_path),
                "bundle_manifest_sha256_at_collection": sha256_path(manifest_path),
                "observation_schema": str(observation_schema_path),
                "observation_schema_member": observation_schema_member,
                "observation_schema_sha256": observation_schema_hash,
                "named_case_manifest": str(named_case_manifest_path),
                "named_case_manifest_member": named_case_manifest_member,
                "named_case_manifest_sha256": named_case_manifest_hash,
                "candidate_report": str(candidate_report_path),
                "candidate_report_member": candidate_member,
                "candidate_report_sha256": candidate_report_hash,
            },
            "provenance": {
                "source_commit": source_commit,
                "candidate_executables": {
                    name: outcome["executable_sha256"] for name, outcome in outcomes.items()
                },
                "candidate_product_versions": {
                    name: outcome["product_version"] for name, outcome in outcomes.items()
                },
                "trusted_driver_reports": [
                    {
                        "candidate": report["candidate_name"],
                        "role": report["role"],
                        "suite_id": report["suite_id"],
                        "schema_version": report["schema_version"],
                        "member": staged_source_reports.get(report["sha256"]),
                        "sha256": report["sha256"],
                        "supplied_named_observations": len(
                            report["payload"].get(observation_field, []) or []
                        ),
                    }
                    for report in driver_reports
                ],
            },
            "anti_reuse": {
                "unique_screenshot_sha256_per_case": True,
                "unique_receipt_sha256_per_case": True,
                "unique_source_report_and_pointer_per_case": True,
                "screenshot_hash_count": len(used_screenshot_hashes),
                "receipt_hash_count": len(used_receipt_hashes),
                "source_observation_count": len(used_observations),
            },
            "bundle_members": bundle_members,
            "summary": {
                "required": 55,
                "collected": collected_count,
                "missing_concrete_driver": len(missing_cases),
                "failed": len(failures),
            },
            "missing_concrete_drivers": missing_cases,
            "cases": case_reports,
            "failures": list(dict.fromkeys(failures)),
        }
        collection_bytes = pretty_json_bytes(collection_report)
        canonical_collection_path = stage_member(staging, collection_member)
        if output_report == canonical_collection_path:
            write_new(output_report, collection_bytes)
        else:
            write_new(canonical_collection_path, collection_bytes)
            write_new(output_report, collection_bytes)
        print(json.dumps(collection_report, ensure_ascii=False, indent=2))
        return 0 if collection_passed else 1
    except (OSError, ValueError, KeyError, RuntimeError, json.JSONDecodeError) as error:
        failure_report = {
            "schema_version": 1,
            "suite_id": COLLECTOR_SUITE,
            "target_release": TARGET_RELEASE,
            "status": "failed",
            "passed": False,
            "summary": {"required": 55, "collected": 0, "missing_concrete_driver": 55, "failed": 1},
            "missing_concrete_drivers": [],
            "cases": case_reports,
            "failures": [f"collector_error:{error}"],
        }
        try:
            if staging.exists() and inside(staging, output_report) and not output_report.exists():
                write_new(output_report, pretty_json_bytes(failure_report))
        except OSError:
            pass
        print(json.dumps(failure_report, ensure_ascii=False, indent=2))
        return 1


if __name__ == "__main__":
    sys.exit(main())
