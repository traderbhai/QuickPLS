"""Fail-closed QuickPLS 3 commercial release-readiness contract validator."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import shutil
import struct
import subprocess
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONTRACT = Path(__file__).with_name("quickpls_3_release_readiness.json")
SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
REQUIREMENT_ID = re.compile(r"^[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+$")
CHECK_ID = re.compile(r"^[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
RECORD_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{11,127}$")
ALLOWED_STATUSES = {"pending", "passed", "failed"}
REQUIRED_CATEGORIES = {
    "signing",
    "installer_lifecycle",
    "updater",
    "support",
    "privacy_security_legal",
    "scientific_review",
    "sbom_provenance",
    "external_beta",
    "release_governance",
}
REQUIRED_IDS = {
    "signing.identity",
    "signing.artifacts",
    "installer.clean_offline",
    "installer.upgrade_recovery",
    "installer.uninstall_deployment",
    "updater.trust_channels",
    "updater.offline_recovery",
    "support.operations",
    "support.docs_diagnostics",
    "trust.privacy_telemetry",
    "trust.security",
    "trust.legal",
    "science.independent_review",
    "supply_chain.sbom_licenses",
    "supply_chain.provenance",
    "beta.cohort_journeys",
    "beta.exit_quality",
    "governance.claims_channels",
}
ALLOWED_EVIDENCE_TYPES = {"file", "url", "attestation"}
MAX_EVIDENCE_AGE = timedelta(days=365)
MAX_FUTURE_SKEW = timedelta(minutes=5)
EXPECTED_IDENTITY = {
    "signing.identity": "artifact_identity",
    "signing.artifacts": "artifact_identity",
    "installer.clean_offline": "artifact_identity",
    "installer.upgrade_recovery": "artifact_identity",
    "installer.uninstall_deployment": "artifact_identity",
    "updater.trust_channels": "artifact_identity",
    "updater.offline_recovery": "artifact_identity",
    "support.operations": "reviewer_identity",
    "support.docs_diagnostics": "reviewer_identity",
    "trust.privacy_telemetry": "reviewer_identity",
    "trust.security": "reviewer_identity",
    "trust.legal": "reviewer_identity",
    "science.independent_review": "reviewer_identity",
    "supply_chain.sbom_licenses": "artifact_identity",
    "supply_chain.provenance": "artifact_identity",
    "beta.cohort_journeys": "reviewer_identity",
    "beta.exit_quality": "reviewer_identity",
    "governance.claims_channels": "reviewer_identity",
}
CANDIDATE_SCOPED_IDS = {
    "signing.artifacts",
    "installer.clean_offline",
    "installer.upgrade_recovery",
    "installer.uninstall_deployment",
    "updater.trust_channels",
    "updater.offline_recovery",
    "supply_chain.sbom_licenses",
    "supply_chain.provenance",
}
REQUIRED_CANDIDATE_ROLES = {
    "desktop",
    "cli",
    "installer",
    "updater_bundle",
    "sbom",
    "provenance",
}
DISTRIBUTION_CANDIDATE_ROLES = {"desktop", "cli", "installer", "updater_bundle"}
SIGNED_PE_ROLES = {"desktop", "cli", "installer"}
MAX_UPDATER_ENTRIES = 8
MAX_UPDATER_UNCOMPRESSED_BYTES = 512 * 1024 * 1024
MAX_UPDATER_COMPRESSION_RATIO = 100
SIGNTOOL_ARGUMENTS = ["verify", "/pa", "/all", "/v", "/tw"]


class ContractError(ValueError):
    """Raised when a readiness contract is structurally invalid."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def _nonempty_string(value: object, label: str) -> str:
    _require(isinstance(value, str) and bool(value.strip()), f"{label} must be a non-empty string")
    return value.strip()


def _validate_timestamp(value: object, label: str) -> datetime:
    text = _nonempty_string(value, label)
    try:
        parsed = datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError as error:
        raise ContractError(f"{label} must be an ISO-8601 timestamp") from error
    _require(parsed.tzinfo is not None, f"{label} must include a timezone")
    return parsed


def _reject_json_constant(value: str) -> None:
    raise ContractError(f"JSON non-finite number is not allowed: {value}")


def _unique_json_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key is not allowed: {key}")
        result[key] = value
    return result


def _strict_json(text: str, label: str) -> object:
    try:
        return json.loads(
            text,
            object_pairs_hook=_unique_json_object,
            parse_constant=_reject_json_constant,
        )
    except ContractError:
        raise
    except json.JSONDecodeError as error:
        raise ContractError(f"{label} is malformed JSON: {error}") from error


def _strict_json_file(path: Path, label: str) -> object:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise ContractError(f"cannot read {label}: {error}") from error
    return _strict_json(text, label)


def _safe_relative_file(value: object, label: str) -> PurePosixPath:
    text = _nonempty_string(value, label).replace("\\", "/")
    path = PurePosixPath(text)
    _require(not path.is_absolute(), f"{label} must be repository-relative")
    _require(".." not in path.parts, f"{label} must not traverse outside the repository")
    _require(path.parts and path.parts[0] not in {"target", "node_modules"}, f"{label} must be durable evidence")
    return path


def _sha256_path(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _canonical_sha256(value: object) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _validate_artifact_descriptor(
    descriptor: object,
    *,
    repository_root: Path,
    label: str,
) -> dict[str, object]:
    _require(isinstance(descriptor, dict), f"{label} must be an object")
    _require(set(descriptor) == {"path", "size", "sha256"}, f"{label} keys are invalid")
    relative = _safe_relative_file(descriptor["path"], f"{label}.path")
    size = descriptor["size"]
    _require(isinstance(size, int) and not isinstance(size, bool) and size > 0, f"{label}.size must be a positive integer")
    digest = _nonempty_string(descriptor["sha256"], f"{label}.sha256")
    _require(bool(SHA256.fullmatch(digest)), f"{label}.sha256 must be lowercase 64-hex SHA-256")
    path = (repository_root / Path(*relative.parts)).resolve()
    root = repository_root.resolve()
    _require(path.is_relative_to(root), f"{label}.path resolves outside the repository")
    _require(path.is_file(), f"{label}.path does not exist: {relative.as_posix()}")
    _require(path.stat().st_size == size, f"{label}.size does not match the artifact bytes")
    _require(_sha256_path(path) == digest, f"{label}.sha256 does not match the artifact bytes")
    return {"path": relative.as_posix(), "size": size, "sha256": digest}


def _descriptor_path(descriptor: dict[str, object], repository_root: Path) -> Path:
    return repository_root / Path(*PurePosixPath(str(descriptor["path"])).parts)


def _validate_windows_pe(path: Path, label: str) -> None:
    data = path.read_bytes()
    _require(len(data) >= 0x200, f"{label} is too small to be a Windows PE executable")
    _require(data[:2] == b"MZ", f"{label} has no DOS MZ header")
    pe_offset = struct.unpack_from("<I", data, 0x3C)[0]
    _require(0x40 <= pe_offset <= len(data) - 24, f"{label} has an invalid PE header offset")
    _require(data[pe_offset : pe_offset + 4] == b"PE\0\0", f"{label} has no PE signature")
    machine, section_count = struct.unpack_from("<HH", data, pe_offset + 4)
    optional_size = struct.unpack_from("<H", data, pe_offset + 20)[0]
    _require(machine == 0x8664, f"{label} is not an x64 Windows PE")
    _require(1 <= section_count <= 96, f"{label} has an invalid PE section count")
    _require(optional_size >= 152, f"{label} optional header cannot contain a security directory")
    optional_offset = pe_offset + 24
    _require(optional_offset + optional_size <= len(data), f"{label} optional header exceeds file bounds")
    _require(struct.unpack_from("<H", data, optional_offset)[0] == 0x20B, f"{label} is not PE32+")
    directory_count = struct.unpack_from("<I", data, optional_offset + 108)[0]
    _require(directory_count >= 5, f"{label} does not declare a certificate-table directory")
    certificate_offset, certificate_size = struct.unpack_from("<II", data, optional_offset + 144)
    _require(certificate_offset > 0 and certificate_size >= 8, f"{label} has no embedded Authenticode certificate table")
    _require(certificate_offset % 8 == 0, f"{label} certificate table offset is not 8-byte aligned")
    _require(certificate_offset + certificate_size <= len(data), f"{label} certificate table exceeds file bounds")
    certificate_length, revision, certificate_type = struct.unpack_from("<IHH", data, certificate_offset)
    _require(8 <= certificate_length <= certificate_size, f"{label} WIN_CERTIFICATE length is invalid")
    _require(revision == 0x0200, f"{label} WIN_CERTIFICATE revision is not 2.0")
    _require(certificate_type == 0x0002, f"{label} WIN_CERTIFICATE type is not PKCS signed data")
    _require(any(data[certificate_offset + 8 : certificate_offset + certificate_length]), f"{label} certificate payload is empty")


def _locate_signtool() -> Path | None:
    for command in ("signtool.exe", "signtool"):
        located = shutil.which(command)
        if located:
            return Path(located).resolve()
    roots = []
    for variable in ("ProgramFiles(x86)", "ProgramFiles"):
        value = os.environ.get(variable)
        if value:
            roots.append(Path(value) / "Windows Kits" / "10" / "bin")
    candidates: list[Path] = []
    for root in roots:
        if root.is_dir():
            candidates.extend(path for path in root.glob("*/x64/signtool.exe") if path.is_file())
    return sorted(candidates, reverse=True)[0].resolve() if candidates else None


def _run_signtool(path: Path) -> dict[str, object]:
    tool = _locate_signtool()
    _require(tool is not None, "Windows SignTool was not found; Authenticode trust cannot be verified")
    command = [str(tool), *SIGNTOOL_ARGUMENTS, str(path.resolve())]
    try:
        completed = subprocess.run(
            command,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=120,
            check=False,
            shell=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise ContractError(f"SignTool execution failed for {path}: {error}") from error
    return {
        "returncode": completed.returncode,
        "stdout": completed.stdout or "",
        "stderr": completed.stderr or "",
    }


def _normalize_signtool_output(stdout: str, stderr: str, path: Path) -> str:
    combined = f"{stdout.rstrip()}\n{stderr.rstrip()}".strip().replace("\r\n", "\n").replace("\r", "\n")
    resolved = str(path.resolve())
    variants = {resolved, resolved.replace("\\", "/"), resolved.replace("/", "\\")}
    for variant in sorted(variants, key=len, reverse=True):
        combined = combined.replace(variant, "<candidate-artifact>")
    return combined


def _parse_signtool_identity(output: str, label: str) -> tuple[str, str]:
    _require(re.search(r"(?im)^\s*Successfully verified:\s*.+$", output) is not None, f"{label} has no successful verification result")
    _require(re.search(r"(?im)^\s*(?:SignTool Error:|Number of errors:\s*[1-9])", output) is None, f"{label} reports an untrusted signature")
    _require("warning" not in output.casefold(), f"{label} reports a verification warning")
    chain = re.search(r"(?is)Signing Certificate Chain:\s*(.*?)(?:Timestamp Verified by:|The signature is timestamped:)", output)
    _require(chain is not None, f"{label} has no signing certificate chain")
    publisher_match = re.search(r"(?im)^\s*Issued to:\s*(.+?)\s*$", chain.group(1))
    _require(publisher_match is not None, f"{label} has no signing publisher")
    timestamp_match = re.search(r"(?im)^\s*The signature is timestamped:\s*(.+?)\s*$", output)
    _require(timestamp_match is not None, f"{label} has no trusted timestamp")
    publisher = publisher_match.group(1).strip()
    timestamp = timestamp_match.group(1).strip()
    _nonempty_string(publisher, f"{label} publisher")
    _nonempty_string(timestamp, f"{label} timestamp")
    return publisher, timestamp


def _verify_authenticode(path: Path, expected_sha256: str, label: str) -> dict[str, object]:
    _require(_sha256_path(path) == expected_sha256, f"{label} file hash changed before SignTool verification")
    execution = _run_signtool(path)
    returncode = execution.get("returncode")
    _require(isinstance(returncode, int) and not isinstance(returncode, bool), f"{label} SignTool return code is invalid")
    stdout = execution.get("stdout")
    stderr = execution.get("stderr")
    _require(isinstance(stdout, str) and isinstance(stderr, str), f"{label} SignTool output is invalid")
    normalized = _normalize_signtool_output(stdout, stderr, path)
    _require(returncode == 0, f"{label} SignTool trust verification failed with exit code {returncode}")
    publisher, timestamp = _parse_signtool_identity(normalized, label)
    _require(_sha256_path(path) == expected_sha256, f"{label} file hash changed during SignTool verification")
    return {
        "command": SIGNTOOL_ARGUMENTS,
        "exit_code": returncode,
        "verification_output": normalized,
        "verification_output_sha256": hashlib.sha256(normalized.encode("utf-8")).hexdigest(),
        "publisher": publisher,
        "timestamp": timestamp,
        "verified_file_sha256": expected_sha256,
    }


def _validate_updater_zip(path: Path, installer: dict[str, object], target_release: str, label: str) -> None:
    try:
        with zipfile.ZipFile(path) as archive:
            entries = archive.infolist()
            _require(1 <= len(entries) <= MAX_UPDATER_ENTRIES, f"{label} entry count is outside the bounded policy")
            _require(archive.testzip() is None, f"{label} contains a corrupt entry")
            total = 0
            names: list[str] = []
            for entry in entries:
                name = entry.filename
                normalized = PurePosixPath(name)
                _require(name and "\\" not in name and ":" not in name, f"{label} contains an unsafe entry name")
                _require(not normalized.is_absolute() and ".." not in normalized.parts, f"{label} contains path traversal")
                _require(not entry.is_dir(), f"{label} must contain files only")
                unix_mode = (entry.external_attr >> 16) & 0xF000
                _require(unix_mode != 0xA000, f"{label} must not contain symbolic links")
                _require(entry.file_size > 0, f"{label} contains an empty payload")
                total += entry.file_size
                _require(total <= MAX_UPDATER_UNCOMPRESSED_BYTES, f"{label} exceeds the uncompressed size limit")
                if entry.compress_size > 0:
                    _require(
                        entry.file_size / entry.compress_size <= MAX_UPDATER_COMPRESSION_RATIO,
                        f"{label} contains an excessive compression ratio",
                    )
                names.append(name)
            expected_name = f"QuickPLS_{target_release}_x64-setup.exe"
            _require(names == [expected_name], f"{label} must contain only {expected_name}")
            payload = archive.read(expected_name)
    except (OSError, zipfile.BadZipFile, RuntimeError) as error:
        raise ContractError(f"{label} is not a valid updater ZIP: {error}") from error
    _require(len(payload) == installer["size"], f"{label} installer payload size does not match the candidate")
    _require(hashlib.sha256(payload).hexdigest() == installer["sha256"], f"{label} installer payload hash does not match the candidate")


def _validate_structured_result(value: object, label: str) -> None:
    _require(isinstance(value, dict), f"{label} must be an object")
    _require(set(value) == {"summary", "measurements"}, f"{label} keys are invalid")
    _nonempty_string(value["summary"], f"{label}.summary")
    measurements = value["measurements"]
    _require(isinstance(measurements, dict) and measurements, f"{label}.measurements must be a non-empty object")
    for key, measurement in measurements.items():
        _nonempty_string(key, f"{label}.measurements key")
        _require(
            isinstance(measurement, (str, int, float, bool)) and measurement is not None,
            f"{label}.measurements.{key} must be a scalar",
        )
        if isinstance(measurement, str):
            _nonempty_string(measurement, f"{label}.measurements.{key}")
        if isinstance(measurement, float):
            _require(math.isfinite(measurement), f"{label}.measurements.{key} must be finite")


def _validate_gate_report(
    descriptor: dict[str, object],
    *,
    requirement_id: str,
    target_release: str,
    candidate_id: str | None,
    criterion_results: list[object],
    repository_root: Path,
    label: str,
) -> None:
    _require(str(descriptor["path"]).lower().endswith(".json"), f"{label} must be a structured JSON report")
    report = _strict_json_file(_descriptor_path(descriptor, repository_root), label)
    _require(isinstance(report, dict), f"{label} must be a JSON object")
    _require(
        set(report) == {"schema_version", "report_type", "requirement_id", "target_release", "candidate_id", "criterion_results"},
        f"{label} keys are invalid",
    )
    _require(report["schema_version"] == 1, f"{label}.schema_version must be 1")
    _require(report["report_type"] == "quickpls_release_gate", f"{label}.report_type is invalid")
    _require(report["requirement_id"] == requirement_id, f"{label}.requirement_id does not match")
    _require(report["target_release"] == target_release, f"{label}.target_release does not match")
    _require(report["candidate_id"] == candidate_id, f"{label}.candidate_id does not match")
    _require(report["criterion_results"] == criterion_results, f"{label}.criterion_results do not match the evidence record")


def _candidate_distribution_identity(target_release: str, artifacts: dict[str, dict[str, object]]) -> str:
    return _canonical_sha256(
        {
            "target_release": target_release,
            "artifacts": [
                {
                    "role": role,
                    "size": artifacts[role]["size"],
                    "sha256": artifacts[role]["sha256"],
                }
                for role in sorted(DISTRIBUTION_CANDIDATE_ROLES)
            ],
        }
    )


def _candidate_digest_map(artifacts: dict[str, dict[str, object]]) -> dict[str, str]:
    return {role: str(artifacts[role]["sha256"]) for role in sorted(DISTRIBUTION_CANDIDATE_ROLES)}


def _validate_signature_report(
    descriptor: dict[str, object],
    *,
    role: str,
    artifact_path: Path,
    artifact_sha256: str,
    target_release: str,
    candidate_id: str,
    repository_root: Path,
    verify_trust: bool,
    authenticode_cache: dict[str, dict[str, object]],
    label: str,
) -> None:
    _require(str(descriptor["path"]).lower().endswith(".json"), f"{label} must be JSON")
    report = _strict_json_file(_descriptor_path(descriptor, repository_root), label)
    _require(isinstance(report, dict), f"{label} must be a JSON object")
    expected = {
        "schema_version",
        "target_release",
        "candidate_id",
        "role",
        "artifact_sha256",
        "authenticode_valid",
        "publisher",
        "timestamp",
        "verification_tool",
        "command",
        "exit_code",
        "verification_output",
        "verification_output_sha256",
        "verified_file_sha256",
        "warnings",
    }
    _require(set(report) == expected, f"{label} keys are invalid")
    _require(report["schema_version"] == 1, f"{label}.schema_version must be 1")
    _require(report["target_release"] == target_release, f"{label}.target_release does not match")
    _require(report["candidate_id"] == candidate_id, f"{label}.candidate_id does not match")
    _require(report["role"] == role, f"{label}.role does not match")
    _require(report["artifact_sha256"] == artifact_sha256, f"{label}.artifact_sha256 does not match")
    _require(report["authenticode_valid"] is True, f"{label}.authenticode_valid must be true")
    _nonempty_string(report["publisher"], f"{label}.publisher")
    _nonempty_string(report["timestamp"], f"{label}.timestamp")
    _require(report["verification_tool"] == "signtool", f"{label}.verification_tool must be signtool")
    _require(report["command"] == SIGNTOOL_ARGUMENTS, f"{label}.command does not use the required SignTool policy")
    _require(report["exit_code"] == 0, f"{label}.exit_code must be zero")
    verification_output = _nonempty_string(report["verification_output"], f"{label}.verification_output")
    verification_output_sha256 = _nonempty_string(
        report["verification_output_sha256"], f"{label}.verification_output_sha256"
    )
    _require(
        bool(SHA256.fullmatch(verification_output_sha256)),
        f"{label}.verification_output_sha256 must be lowercase 64-hex SHA-256",
    )
    _require(
        hashlib.sha256(verification_output.encode("utf-8")).hexdigest() == verification_output_sha256,
        f"{label}.verification_output_sha256 does not match the recorded output",
    )
    _require(report["verified_file_sha256"] == artifact_sha256, f"{label}.verified_file_sha256 does not match")
    _require(report["warnings"] == [], f"{label}.warnings must be empty")
    if verify_trust:
        cache_key = f"{artifact_path.resolve()}:{artifact_sha256}"
        actual = authenticode_cache.get(cache_key)
        if actual is None:
            actual = _verify_authenticode(artifact_path, artifact_sha256, label)
            authenticode_cache[cache_key] = actual
        for field in (
            "command",
            "exit_code",
            "verification_output",
            "verification_output_sha256",
            "publisher",
            "timestamp",
            "verified_file_sha256",
        ):
            _require(report[field] == actual[field], f"{label}.{field} does not match validation-time SignTool output")


def _validate_sbom(
    path: Path,
    *,
    target_release: str,
    candidate_id: str,
    artifact_digests: dict[str, str],
    label: str,
) -> None:
    sbom = _strict_json_file(path, label)
    _require(isinstance(sbom, dict), f"{label} must be a JSON object")
    expected = {
        "schema_version",
        "document_type",
        "target_release",
        "candidate_id",
        "candidate_artifact_digests",
        "format",
        "spec_version",
        "components",
    }
    _require(set(sbom) == expected, f"{label} keys are invalid")
    _require(sbom["schema_version"] == 1, f"{label}.schema_version must be 1")
    _require(sbom["document_type"] == "quickpls_sbom", f"{label}.document_type is invalid")
    _require(sbom["target_release"] == target_release, f"{label}.target_release does not match")
    _require(sbom["candidate_id"] == candidate_id, f"{label}.candidate_id does not match")
    _require(sbom["candidate_artifact_digests"] == artifact_digests, f"{label} artifact digests do not match")
    _require(sbom["format"] == "CycloneDX", f"{label}.format must be CycloneDX")
    _require(sbom["spec_version"] == "1.6", f"{label}.spec_version must be 1.6")
    components = sbom["components"]
    _require(isinstance(components, list) and components, f"{label}.components must be non-empty")
    for index, component in enumerate(components):
        component_label = f"{label}.components[{index}]"
        _require(isinstance(component, dict), f"{component_label} must be an object")
        _require(set(component) == {"type", "name", "version", "licenses"}, f"{component_label} keys are invalid")
        _require(component["type"] in {"application", "library"}, f"{component_label}.type is invalid")
        _nonempty_string(component["name"], f"{component_label}.name")
        _nonempty_string(component["version"], f"{component_label}.version")
        licenses = component["licenses"]
        _require(isinstance(licenses, list) and licenses, f"{component_label}.licenses must be non-empty")
        for license_index, license_name in enumerate(licenses):
            _nonempty_string(license_name, f"{component_label}.licenses[{license_index}]")


def _validate_provenance(
    path: Path,
    *,
    target_release: str,
    candidate_id: str,
    artifact_digests: dict[str, str],
    sbom_sha256: str,
    label: str,
) -> None:
    provenance = _strict_json_file(path, label)
    _require(isinstance(provenance, dict), f"{label} must be a JSON object")
    expected = {
        "schema_version",
        "document_type",
        "target_release",
        "candidate_id",
        "candidate_artifact_digests",
        "source_commit",
        "build_id",
        "builder_identity",
        "build_started_at",
        "build_finished_at",
        "sbom_sha256",
    }
    _require(set(provenance) == expected, f"{label} keys are invalid")
    _require(provenance["schema_version"] == 1, f"{label}.schema_version must be 1")
    _require(provenance["document_type"] == "quickpls_release_provenance", f"{label}.document_type is invalid")
    _require(provenance["target_release"] == target_release, f"{label}.target_release does not match")
    _require(provenance["candidate_id"] == candidate_id, f"{label}.candidate_id does not match")
    _require(provenance["candidate_artifact_digests"] == artifact_digests, f"{label} artifact digests do not match")
    commit = _nonempty_string(provenance["source_commit"], f"{label}.source_commit")
    _require(bool(re.fullmatch(r"[0-9a-f]{40}", commit)), f"{label}.source_commit must be lowercase 40-hex")
    _nonempty_string(provenance["build_id"], f"{label}.build_id")
    _nonempty_string(provenance["builder_identity"], f"{label}.builder_identity")
    started = _validate_timestamp(provenance["build_started_at"], f"{label}.build_started_at")
    finished = _validate_timestamp(provenance["build_finished_at"], f"{label}.build_finished_at")
    _require(finished >= started, f"{label}.build_finished_at precedes build_started_at")
    _require(provenance["sbom_sha256"] == sbom_sha256, f"{label}.sbom_sha256 does not match the candidate SBOM")


def _validate_candidate_manifest(
    descriptor: object,
    *,
    candidate_id: object,
    target_release: str,
    repository_root: Path,
    verify_authenticode: bool,
    authenticode_cache: dict[str, dict[str, object]],
    label: str,
) -> dict[str, object]:
    candidate = _nonempty_string(candidate_id, f"{label}.candidate_id")
    _require(bool(SHA256.fullmatch(candidate)), f"{label}.candidate_id must be lowercase 64-hex SHA-256")
    manifest_descriptor = _validate_artifact_descriptor(
        descriptor,
        repository_root=repository_root,
        label=f"{label}.candidate_manifest",
    )
    manifest_path = repository_root / Path(*PurePosixPath(str(manifest_descriptor["path"])).parts)
    manifest = _strict_json_file(manifest_path, f"{label}.candidate_manifest document")
    _require(isinstance(manifest, dict), f"{label}.candidate_manifest document must be an object")
    _require(
        set(manifest) == {"schema_version", "target_release", "candidate_id", "artifacts", "signature_evidence"},
        f"{label}.candidate_manifest document keys are invalid",
    )
    _require(manifest["schema_version"] == 1, f"{label}.candidate_manifest schema_version must be 1")
    _require(manifest["target_release"] == target_release, f"{label}.candidate_manifest target_release does not match")
    _require(manifest["candidate_id"] == candidate, f"{label}.candidate_manifest candidate_id does not match")
    artifacts = manifest["artifacts"]
    _require(isinstance(artifacts, list) and artifacts, f"{label}.candidate_manifest artifacts must be non-empty")
    roles: list[str] = []
    paths: list[str] = []
    validated_artifacts: list[dict[str, object]] = []
    for index, item in enumerate(artifacts):
        artifact_label = f"{label}.candidate_manifest.artifacts[{index}]"
        _require(isinstance(item, dict), f"{artifact_label} must be an object")
        _require(set(item) == {"role", "path", "size", "sha256"}, f"{artifact_label} keys are invalid")
        role = _nonempty_string(item["role"], f"{artifact_label}.role")
        roles.append(role)
        artifact = _validate_artifact_descriptor(
            {"path": item["path"], "size": item["size"], "sha256": item["sha256"]},
            repository_root=repository_root,
            label=artifact_label,
        )
        paths.append(str(artifact["path"]).casefold())
        validated_artifacts.append({"role": role, **artifact})
    _require(len(roles) == len(set(roles)), f"{label}.candidate_manifest artifact roles must be unique")
    _require(len(paths) == len(set(paths)), f"{label}.candidate_manifest artifact paths must be unique")
    _require(set(roles) == REQUIRED_CANDIDATE_ROLES, f"{label}.candidate_manifest artifact role set is incomplete")
    artifact_map = {str(item["role"]): {key: value for key, value in item.items() if key != "role"} for item in validated_artifacts}
    expected_candidate = _candidate_distribution_identity(target_release, artifact_map)
    _require(candidate == expected_candidate, f"{label}.candidate_id does not match the distribution artifact set")

    for role in sorted(SIGNED_PE_ROLES):
        _validate_windows_pe(_descriptor_path(artifact_map[role], repository_root), f"{label}.{role}")
    _validate_updater_zip(
        _descriptor_path(artifact_map["updater_bundle"], repository_root),
        artifact_map["installer"],
        target_release,
        f"{label}.updater_bundle",
    )

    signature_evidence = manifest["signature_evidence"]
    _require(isinstance(signature_evidence, list), f"{label}.candidate_manifest signature_evidence must be a list")
    _require(len(signature_evidence) == len(SIGNED_PE_ROLES), f"{label}.candidate_manifest signature evidence is incomplete")
    signature_roles: list[str] = []
    signature_paths: list[str] = []
    validated_signatures: list[dict[str, object]] = []
    for index, item in enumerate(signature_evidence):
        signature_label = f"{label}.candidate_manifest.signature_evidence[{index}]"
        _require(isinstance(item, dict), f"{signature_label} must be an object")
        _require(set(item) == {"role", "artifact_sha256", "report"}, f"{signature_label} keys are invalid")
        role = _nonempty_string(item["role"], f"{signature_label}.role")
        _require(role in SIGNED_PE_ROLES, f"{signature_label}.role is not a signed PE role")
        artifact_sha256 = _nonempty_string(item["artifact_sha256"], f"{signature_label}.artifact_sha256")
        _require(artifact_sha256 == artifact_map[role]["sha256"], f"{signature_label} is bound to the wrong artifact")
        report_descriptor = _validate_artifact_descriptor(
            item["report"],
            repository_root=repository_root,
            label=f"{signature_label}.report",
        )
        _validate_signature_report(
            report_descriptor,
            role=role,
            artifact_path=_descriptor_path(artifact_map[role], repository_root),
            artifact_sha256=artifact_sha256,
            target_release=target_release,
            candidate_id=candidate,
            repository_root=repository_root,
            verify_trust=verify_authenticode,
            authenticode_cache=authenticode_cache,
            label=f"{signature_label}.report document",
        )
        signature_roles.append(role)
        signature_paths.append(str(report_descriptor["path"]).casefold())
        validated_signatures.append(
            {"role": role, "artifact_sha256": artifact_sha256, "report": report_descriptor}
        )
    _require(len(signature_roles) == len(set(signature_roles)), f"{label}.candidate_manifest signature roles must be unique")
    _require(set(signature_roles) == SIGNED_PE_ROLES, f"{label}.candidate_manifest signature roles are incomplete")
    _require(len(signature_paths) == len(set(signature_paths)), f"{label}.candidate_manifest signature report paths must be unique")

    distribution_digests = _candidate_digest_map(artifact_map)
    _validate_sbom(
        _descriptor_path(artifact_map["sbom"], repository_root),
        target_release=target_release,
        candidate_id=candidate,
        artifact_digests=distribution_digests,
        label=f"{label}.sbom",
    )
    _validate_provenance(
        _descriptor_path(artifact_map["provenance"], repository_root),
        target_release=target_release,
        candidate_id=candidate,
        artifact_digests=distribution_digests,
        sbom_sha256=str(artifact_map["sbom"]["sha256"]),
        label=f"{label}.provenance",
    )
    return {
        "candidate_id": candidate,
        "candidate_manifest": manifest_descriptor,
        "artifacts": sorted(validated_artifacts, key=lambda item: str(item["role"])),
        "signature_evidence": sorted(validated_signatures, key=lambda item: str(item["role"])),
    }


def _validate_criterion_results(value: object, expected_ids: list[str], label: str) -> None:
    _require(isinstance(value, list), f"{label} must be a list")
    _require(len(value) == len(expected_ids), f"{label} must contain one result for every acceptance check")
    found: list[str] = []
    for index, row in enumerate(value):
        row_label = f"{label}[{index}]"
        _require(isinstance(row, dict), f"{row_label} must be an object")
        _require(set(row) == {"check_id", "passed", "result"}, f"{row_label} keys are invalid")
        check_id = _nonempty_string(row["check_id"], f"{row_label}.check_id")
        _require(bool(CHECK_ID.fullmatch(check_id)), f"{row_label}.check_id has invalid syntax")
        _require(row["passed"] is True, f"{row_label}.passed must be true")
        _validate_structured_result(row["result"], f"{row_label}.result")
        found.append(check_id)
    _require(len(found) == len(set(found)), f"{label} check IDs must be unique")
    _require(set(found) == set(expected_ids), f"{label} does not bind the contract acceptance checks")


def _validate_reviewer_identity(value: object, requirement_id: str, label: str) -> None:
    _require(isinstance(value, dict), f"{label} must be an object")
    expected = {
        "name",
        "role",
        "organization",
        "independence",
        "conflict_disclosure",
        "disposition",
        "reviewed_scope",
        "record_id",
    }
    _require(set(value) == expected, f"{label} keys are invalid")
    for key in expected - {"independence", "disposition", "record_id"}:
        _nonempty_string(value[key], f"{label}.{key}")
    independence = _nonempty_string(value["independence"], f"{label}.independence")
    _require(
        independence in {"independent", "organizationally_separate", "accountable_owner"},
        f"{label}.independence is not recognized",
    )
    if requirement_id == "science.independent_review":
        _require(independence == "independent", f"{label}.independence must be independent")
    disposition = _nonempty_string(value["disposition"], f"{label}.disposition")
    _require(
        disposition in {"approved", "approved_with_closed_findings"},
        f"{label}.disposition does not permit release",
    )
    record_id = _nonempty_string(value["record_id"], f"{label}.record_id")
    _require(bool(RECORD_ID.fullmatch(record_id)), f"{label}.record_id is not a stable record identifier")


def _validate_evidence(
    evidence: object,
    *,
    requirement_id: str,
    target_release: str,
    acceptance_check_ids: list[str],
    repository_root: Path,
    seen_evidence_files: dict[str, str],
    candidate_state: dict[str, object],
    evidence_bindings: list[dict[str, str]],
    verify_authenticode: bool,
    authenticode_cache: dict[str, dict[str, object]],
    now: datetime,
) -> datetime | None:
    _require(isinstance(evidence, list), f"{requirement_id}.evidence must be a list")
    latest: datetime | None = None
    for index, item in enumerate(evidence):
        label = f"{requirement_id}.evidence[{index}]"
        _require(isinstance(item, dict), f"{label} must be an object")
        _require(set(item) == {"type", "ref"}, f"{label} must contain exactly type and ref")
        kind = _nonempty_string(item.get("type"), f"{label}.type")
        ref = _nonempty_string(item.get("ref"), f"{label}.ref")
        _require(kind in ALLOWED_EVIDENCE_TYPES, f"{label}.type is unsupported: {kind}")
        if kind == "file":
            relative = _safe_relative_file(ref, f"{label}.ref")
            _require(relative.suffix.lower() == ".json", f"{label}.ref must be a JSON evidence record")
            evidence_key = relative.as_posix().casefold()
            previous = seen_evidence_files.get(evidence_key)
            _require(
                previous is None,
                f"{label}.ref is already assigned to {previous}; evidence records cannot be reused",
            )
            seen_evidence_files[evidence_key] = requirement_id

            candidate = (repository_root / Path(*relative.parts)).resolve()
            root = repository_root.resolve()
            _require(candidate.is_relative_to(root), f"{label}.ref resolves outside the repository")
            _require(candidate.is_file(), f"{label}.ref does not exist: {relative.as_posix()}")
            record = _strict_json_file(candidate, f"evidence record {relative.as_posix()}")
            performed_at, candidate_binding = _validate_evidence_record(
                record,
                requirement_id=requirement_id,
                target_release=target_release,
                acceptance_check_ids=acceptance_check_ids,
                expected_identity=EXPECTED_IDENTITY[requirement_id],
                repository_root=repository_root,
                verify_authenticode=verify_authenticode,
                authenticode_cache=authenticode_cache,
                now=now,
                label=f"evidence record {relative.as_posix()}",
            )
            latest = performed_at if latest is None or performed_at > latest else latest
            evidence_bindings.append(
                {
                    "requirement_id": requirement_id,
                    "path": relative.as_posix(),
                    "sha256": _sha256_path(candidate),
                }
            )
            if candidate_binding is not None:
                existing = candidate_state.get("binding")
                if existing is None:
                    candidate_state["binding"] = candidate_binding
                else:
                    _require(existing == candidate_binding, f"{requirement_id} evidence is bound to a different release candidate")
        elif kind == "url":
            _require(ref.startswith("https://"), f"{label}.ref must be an HTTPS URL")
        else:
            _require(len(ref) >= 12, f"{label}.ref must be a stable review or audit reference")
    return latest


def _validate_evidence_record(
    record: object,
    *,
    requirement_id: str,
    target_release: str,
    acceptance_check_ids: list[str],
    expected_identity: str,
    repository_root: Path,
    verify_authenticode: bool,
    authenticode_cache: dict[str, dict[str, object]],
    now: datetime,
    label: str,
) -> tuple[datetime, dict[str, object] | None]:
    _require(isinstance(record, dict), f"{label} must be a JSON object")
    base_keys = {
        "schema_version",
        "requirement_id",
        "target_release",
        "passed",
        "performed_at",
        "scope",
        "summary",
        "criterion_results",
    }
    candidate_keys = {"candidate_id", "candidate_manifest"} if requirement_id in CANDIDATE_SCOPED_IDS else set()
    _require(
        set(record) == base_keys | {expected_identity} | candidate_keys,
        f"{label} keys do not match its requirement-specific evidence schema",
    )
    _require(record["schema_version"] == 1, f"{label}.schema_version must be 1")
    _require(record["requirement_id"] == requirement_id, f"{label}.requirement_id does not match {requirement_id}")
    _require(record["target_release"] == target_release, f"{label}.target_release does not match {target_release}")
    _require(record["passed"] is True, f"{label}.passed must be true")
    performed_at = _validate_timestamp(record["performed_at"], f"{label}.performed_at")
    performed_utc = performed_at.astimezone(timezone.utc)
    now_utc = now.astimezone(timezone.utc)
    _require(performed_utc <= now_utc + MAX_FUTURE_SKEW, f"{label}.performed_at is in the future")
    _require(now_utc - performed_utc <= MAX_EVIDENCE_AGE, f"{label}.performed_at is stale")
    _nonempty_string(record["scope"], f"{label}.scope")
    _nonempty_string(record["summary"], f"{label}.summary")
    _validate_criterion_results(record["criterion_results"], acceptance_check_ids, f"{label}.criterion_results")

    record_candidate_id = record["candidate_id"] if requirement_id in CANDIDATE_SCOPED_IDS else None
    identity = record[expected_identity]
    if expected_identity == "artifact_identity":
        _require(isinstance(identity, dict), f"{label}.artifact_identity must be an object")
        _require(set(identity) == {"name", "identifier", "artifact"}, f"{label}.artifact_identity keys are invalid")
        _nonempty_string(identity["name"], f"{label}.artifact_identity.name")
        identifier = _nonempty_string(identity["identifier"], f"{label}.artifact_identity.identifier")
        _require(bool(SHA256.fullmatch(identifier)), f"{label}.artifact_identity.identifier must be lowercase 64-hex SHA-256")
        artifact = _validate_artifact_descriptor(
            identity["artifact"],
            repository_root=repository_root,
            label=f"{label}.artifact_identity.artifact",
        )
        _require(identifier == artifact["sha256"], f"{label}.artifact_identity.identifier does not match its artifact")
        _validate_gate_report(
            artifact,
            requirement_id=requirement_id,
            target_release=target_release,
            candidate_id=record_candidate_id,
            criterion_results=record["criterion_results"],
            repository_root=repository_root,
            label=f"{label}.artifact_identity report",
        )
    else:
        _validate_reviewer_identity(identity, requirement_id, f"{label}.reviewer_identity")

    candidate_binding = None
    if requirement_id in CANDIDATE_SCOPED_IDS:
        candidate_binding = _validate_candidate_manifest(
            record["candidate_manifest"],
            candidate_id=record_candidate_id,
            target_release=target_release,
            repository_root=repository_root,
            verify_authenticode=verify_authenticode,
            authenticode_cache=authenticode_cache,
            label=label,
        )
    return performed_at, candidate_binding


def validate_contract(
    contract: object,
    *,
    repository_root: Path = ROOT,
    now: datetime | None = None,
) -> dict[str, Any]:
    """Validate structure and return a computed readiness report.

    `passed` is not accepted on assertion alone: every passed requirement must
    cite a unique, existing, strict JSON repository evidence record bound to
    the exact requirement and target release. External URLs and attestations
    may supplement that record but cannot replace it.
    """

    _require(isinstance(contract, dict), "contract must be a JSON object")
    expected_top = {
        "schema_version",
        "program",
        "target_release",
        "target_channel",
        "product_policy",
        "release_channels",
        "evidence_policy",
        "overall_status",
        "release_decision",
        "requirements",
    }
    _require(set(contract) == expected_top, "contract top-level keys do not match schema version 1")
    _require(contract["schema_version"] == 1, "schema_version must be 1")
    _require(contract["program"] == "quickpls_3_competitor_release", "program is not recognized")
    release = _nonempty_string(contract["target_release"], "target_release")
    _require(bool(SEMVER.fullmatch(release)), "target_release must be semantic version syntax")
    _require(contract["target_channel"] == "stable", "target_channel must be stable")
    _require(contract["overall_status"] in ALLOWED_STATUSES, "overall_status is invalid")
    validation_time = now or datetime.now(timezone.utc)
    _require(validation_time.tzinfo is not None, "validation time must include a timezone")

    policy = contract["product_policy"]
    _require(isinstance(policy, dict), "product_policy must be an object")
    _require(policy.get("platform") == "windows_x64", "platform must be windows_x64")
    _require(policy.get("offline_default") is True, "offline_default must remain true")
    _require(policy.get("telemetry_default") == "disabled", "telemetry_default must remain disabled")
    _require(policy.get("update_check_default") == "manual", "update_check_default must remain manual")
    _require(
        policy.get("positioning") == "competitor_for_documented_supported_workflows",
        "positioning must remain bounded to documented supported workflows",
    )
    prohibited = policy.get("prohibited_claims")
    _require(isinstance(prohibited, list) and len(prohibited) >= 4, "prohibited_claims must retain all claim guards")
    _require(len(prohibited) == len(set(prohibited)), "prohibited_claims must be unique")

    channels = contract["release_channels"]
    _require(
        channels
        == {
            "internal": "maintainer_only",
            "beta": "signed_prerelease",
            "stable": "all_mandatory_gates_passed",
        },
        "release_channels do not match the frozen channel policy",
    )
    evidence_policy = contract["evidence_policy"]
    _require(
        evidence_policy
        == {
            "record_schema_version": 1,
            "max_age_days": MAX_EVIDENCE_AGE.days,
            "repository_json_required": True,
            "requirement_and_release_binding": True,
            "identity_binding": "requirement_specific_artifact_or_reviewer",
            "artifact_rehash_required": True,
            "candidate_semantic_validation": True,
            "pe_authenticode_binding": True,
            "authenticode_validation_time": "signtool_verify_pa_all_v_tw",
            "authenticode_runtime_evidence_binding": True,
            "authenticode_bypass": "prohibited",
            "updater_zip_safety": True,
            "sbom_provenance_binding": True,
            "candidate_manifest_binding": True,
            "criterion_result_binding": "structured_json",
            "approval_after_evidence": True,
            "reuse_across_requirements": "prohibited",
            "supplementary_types": ["url", "attestation"],
        },
        "evidence_policy does not match the fail-closed evidence contract",
    )

    requirements = contract["requirements"]
    _require(isinstance(requirements, list) and requirements, "requirements must be a non-empty list")
    ids: list[str] = []
    categories: set[str] = set()
    pending: list[str] = []
    failed: list[str] = []
    passed: list[str] = []
    seen_evidence_files: dict[str, str] = {}
    candidate_state: dict[str, object] = {}
    evidence_bindings: list[dict[str, str]] = []
    authenticode_cache: dict[str, dict[str, object]] = {}
    latest_evidence_at: datetime | None = None
    all_check_ids: list[str] = []
    for index, item in enumerate(requirements):
        label = f"requirements[{index}]"
        _require(isinstance(item, dict), f"{label} must be an object")
        _require(
            set(item)
            == {
                "id",
                "category",
                "owner",
                "required",
                "status",
                "evidence_identity",
                "acceptance_check_ids",
                "acceptance_criteria",
                "evidence",
            },
            f"{label} keys do not match the requirement schema",
        )
        requirement_id = _nonempty_string(item["id"], f"{label}.id")
        _require(bool(REQUIREMENT_ID.fullmatch(requirement_id)), f"{label}.id has invalid syntax")
        expected_identity = EXPECTED_IDENTITY.get(requirement_id)
        _require(expected_identity is not None, f"{label}.id is not a recognized readiness requirement")
        _require(
            item["evidence_identity"] == expected_identity,
            f"{requirement_id}.evidence_identity must be {expected_identity}",
        )
        ids.append(requirement_id)
        category = _nonempty_string(item["category"], f"{label}.category")
        categories.add(category)
        _require(item["required"] is True, f"{requirement_id} must remain release-blocking")
        _nonempty_string(item["owner"], f"{requirement_id}.owner")
        status = item["status"]
        _require(status in ALLOWED_STATUSES, f"{requirement_id}.status is invalid")
        check_ids = item["acceptance_check_ids"]
        criteria = item["acceptance_criteria"]
        _require(isinstance(criteria, list) and len(criteria) >= 2, f"{requirement_id} needs at least two acceptance criteria")
        _require(
            isinstance(check_ids, list) and len(check_ids) == len(criteria),
            f"{requirement_id}.acceptance_check_ids must align one-to-one with acceptance_criteria",
        )
        for check_index, check_id in enumerate(check_ids):
            checked = _nonempty_string(check_id, f"{requirement_id}.acceptance_check_ids[{check_index}]")
            _require(bool(CHECK_ID.fullmatch(checked)), f"{requirement_id} acceptance check ID has invalid syntax")
            _require(checked.startswith(f"{requirement_id}."), f"{requirement_id} acceptance check ID must be requirement-scoped")
            all_check_ids.append(checked)
        for criterion_index, criterion in enumerate(criteria):
            _nonempty_string(criterion, f"{requirement_id}.acceptance_criteria[{criterion_index}]")
        evidence = item["evidence"]
        requirement_latest = _validate_evidence(
            evidence,
            requirement_id=requirement_id,
            target_release=release,
            acceptance_check_ids=check_ids,
            repository_root=repository_root,
            seen_evidence_files=seen_evidence_files,
            candidate_state=candidate_state,
            evidence_bindings=evidence_bindings,
            verify_authenticode=status == "passed" and requirement_id in CANDIDATE_SCOPED_IDS,
            authenticode_cache=authenticode_cache,
            now=validation_time,
        )
        if requirement_latest is not None and (
            latest_evidence_at is None or requirement_latest > latest_evidence_at
        ):
            latest_evidence_at = requirement_latest
        if status == "passed":
            _require(evidence, f"{requirement_id} is passed without evidence")
            _require(
                any(record.get("type") == "file" for record in evidence),
                f"{requirement_id} is passed without a repository JSON evidence record",
            )
            passed.append(requirement_id)
        elif status == "failed":
            failed.append(requirement_id)
        else:
            pending.append(requirement_id)

    _require(len(ids) == len(set(ids)), "requirement ids must be unique")
    _require(len(all_check_ids) == len(set(all_check_ids)), "acceptance check IDs must be globally unique")
    _require(set(ids) == REQUIRED_IDS, "required readiness ids are missing or unknown ids were added")
    _require(categories == REQUIRED_CATEGORIES, "required readiness categories are incomplete or unknown")

    ordered_evidence_bindings = sorted(evidence_bindings, key=lambda item: (item["requirement_id"], item["path"]))
    candidate_binding = candidate_state.get("binding")
    candidate_id = candidate_binding.get("candidate_id") if isinstance(candidate_binding, dict) else None
    contract_inputs = {
        "schema_version": contract["schema_version"],
        "target_release": release,
        "target_channel": contract["target_channel"],
        "product_policy": policy,
        "release_channels": channels,
        "evidence_policy": evidence_policy,
        "requirements": [
            {
                "id": item["id"],
                "status": item["status"],
                "acceptance_check_ids": item["acceptance_check_ids"],
                "evidence": item["evidence"],
            }
            for item in requirements
        ],
    }
    competitor_ready_inputs = {
        "target_channel": "stable",
        "required_requirement_ids": sorted(ids),
        "evidence_records": ordered_evidence_bindings,
        "candidate_id": candidate_id,
        "contract_inputs_sha256": _canonical_sha256(contract_inputs),
    }

    decision = contract["release_decision"]
    _require(isinstance(decision, dict), "release_decision must be an object")
    _require(
        set(decision) == {"status", "approved_by", "approved_at", "record"},
        "release_decision keys do not match schema version 1",
    )
    _require(decision["status"] in {"pending", "approved", "rejected"}, "release_decision.status is invalid")
    decision_approved = decision["status"] == "approved"
    if decision_approved:
        _require(not pending and not failed, "release approval requires every mandatory requirement to pass")
        _require(candidate_id is not None, "release approval requires one bound release candidate")
        _nonempty_string(decision["approved_by"], "release_decision.approved_by")
        approved_at = _validate_timestamp(decision["approved_at"], "release_decision.approved_at")
        _require(
            approved_at.astimezone(timezone.utc) <= validation_time.astimezone(timezone.utc) + MAX_FUTURE_SKEW,
            "release_decision.approved_at is in the future",
        )
        _require(latest_evidence_at is not None, "release approval requires completed evidence")
        _require(
            approved_at.astimezone(timezone.utc) >= latest_evidence_at.astimezone(timezone.utc),
            "release_decision.approved_at must not precede the latest evidence",
        )
        decision_record = _safe_relative_file(decision["record"], "release_decision.record")
        _require(decision_record.suffix.lower() == ".json", "release_decision.record must be JSON")
        _require(
            decision_record.as_posix().casefold() not in seen_evidence_files,
            "release_decision.record must be distinct from requirement evidence records",
        )
        record_path = (repository_root / Path(*decision_record.parts)).resolve()
        _require(record_path.is_file(), f"release_decision.record does not exist: {decision_record.as_posix()}")
        decision_evidence = _strict_json_file(record_path, f"release decision {decision_record.as_posix()}")
        _require(isinstance(decision_evidence, dict), "release decision record must be a JSON object")
        _require(
            set(decision_evidence)
            == {
                "schema_version",
                "target_release",
                "approved",
                "approved_by",
                "approved_at",
                "summary",
                "competitor_ready_inputs",
            },
            "release decision record keys do not match schema version 1",
        )
        _require(decision_evidence["schema_version"] == 1, "release decision record schema_version must be 1")
        _require(decision_evidence["target_release"] == release, "release decision target_release does not match")
        _require(decision_evidence["approved"] is True, "release decision record approved must be true")
        _require(decision_evidence["approved_by"] == decision["approved_by"], "release decision approver does not match")
        _require(decision_evidence["approved_at"] == decision["approved_at"], "release decision timestamp does not match")
        _nonempty_string(decision_evidence["summary"], "release decision record summary")
        _require(
            decision_evidence["competitor_ready_inputs"] == competitor_ready_inputs,
            "release decision record does not bind the current competitor-ready inputs",
        )
    else:
        _require(decision["approved_by"] is None, "unapproved release_decision.approved_by must be null")
        _require(decision["approved_at"] is None, "unapproved release_decision.approved_at must be null")
        _require(decision["record"] is None, "unapproved release_decision.record must be null")

    computed_ready = not pending and not failed and decision_approved
    if computed_ready:
        _require(contract["overall_status"] == "passed", "ready contract overall_status must be passed")
    else:
        _require(contract["overall_status"] != "passed", "overall_status cannot pass before every gate and approval")

    return {
        "schema_version": 1,
        "target_release": release,
        "structurally_valid": True,
        "competitor_ready": computed_ready,
        "release_ready": computed_ready,
        "counts": {"passed": len(passed), "pending": len(pending), "failed": len(failed)},
        "pending": pending,
        "failed": failed,
        "release_decision": decision["status"],
    }


def load_and_validate(
    contract_path: Path = DEFAULT_CONTRACT,
    *,
    repository_root: Path = ROOT,
    now: datetime | None = None,
) -> dict[str, Any]:
    try:
        contract = _strict_json_file(contract_path, f"readiness contract {contract_path}")
    except ContractError:
        raise
    except OSError as error:
        raise ContractError(f"cannot load readiness contract {contract_path}: {error}") from error
    return validate_contract(
        contract,
        repository_root=repository_root,
        now=now,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    parser.add_argument(
        "--require-ready",
        action="store_true",
        help="Fail unless every mandatory requirement has evidence and the release decision is approved.",
    )
    args = parser.parse_args()
    try:
        report = load_and_validate(args.contract, repository_root=args.repo_root)
    except ContractError as error:
        print(
            json.dumps(
                {
                    "structurally_valid": False,
                    "competitor_ready": False,
                    "release_ready": False,
                    "error": str(error),
                },
                indent=2,
            )
        )
        return 2
    print(json.dumps(report, indent=2))
    if args.require_ready and not report["release_ready"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
