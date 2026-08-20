#!/usr/bin/env python3
"""Compose fail-closed Rank 0 installed+portable evidence into the release report.

The browser driver and package supervisor emit raw observations.  This module
reopens the actual schema-6 archive, reconciles its canonical document to the
exact Registry cell, hashes all six exported files, normalizes six distinct
evidence roles, and only then writes a whole-Rank or cell-atomic report accepted by
``general_sem_rank0_packaged_acceptance.py``.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Mapping, Sequence

try:
    from validation.general_sem_rank0_packaged_acceptance import (
        DEFAULT_CONTRACT,
        DEFAULT_REGISTRY,
        EXPORT_FORMATS,
        REQUIRED_ARTIFACT_KINDS,
        ContractError,
        load_json,
        qualification_contract_authorities,
        unified_rank0_source_receipt,
        validate_cell_report,
        validate_contract,
        validate_report,
        variant_canonical_authority,
    )
    from validation.general_sem_rank0_export_semantic_readback import semantic_readback
except ModuleNotFoundError:
    from general_sem_rank0_packaged_acceptance import (
        DEFAULT_CONTRACT,
        DEFAULT_REGISTRY,
        EXPORT_FORMATS,
        REQUIRED_ARTIFACT_KINDS,
        ContractError,
        load_json,
        qualification_contract_authorities,
        unified_rank0_source_receipt,
        validate_cell_report,
        validate_contract,
        validate_report,
        variant_canonical_authority,
    )
    from general_sem_rank0_export_semantic_readback import semantic_readback


ROOT = Path(__file__).resolve().parents[1]
SHA256 = re.compile(r"^[0-9a-f]{64}$")
SCALES = (100, 125, 150, 200)
VIEWPORTS = ("1024x700", "1280x720", "1440x900")
PACKAGE_PROVENANCE = {
    "evidence_kind": "windows_pe_package_identity_v1",
    "file_identity_source": "resolved_path_size_sha256",
    "version_identity_source": "System.Diagnostics.FileVersionInfo",
}
TAURI_PORTABLE_BUNDLE_MARKER = b"__TAURI_BUNDLE_TYPE_VAR_UNK"
TAURI_NSIS_BUNDLE_MARKER = b"__TAURI_BUNDLE_TYPE_VAR_NSS"
PACKAGED_TAURI_ORIGIN = "http://tauri.localhost"
PACKAGED_TAURI_IPC_ORIGIN = "http://ipc.localhost"
ALLOWED_OFFLINE_ORIGINS = {
    PACKAGED_TAURI_ORIGIN,
    PACKAGED_TAURI_IPC_ORIGIN,
    "null",
    None,
}


def _exact(value: Any, keys: set[str], subject: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping) or set(value) != keys:
        raise ContractError(f"{subject} fields are not exact")
    return value


def _write_new_json(path: Path, value: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = (
        json.dumps(
            value, indent=2, sort_keys=False, ensure_ascii=False, allow_nan=False
        )
        + "\n"
    )
    with path.open("x", encoding="utf-8", newline="\n") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _is_exact_tauri_nsis_bundle_variant(
    installed_executable: Path, portable_executable: Path
) -> bool:
    """Accept only Tauri's exact UNK -> NSS package-kind marker rewrite."""

    portable = portable_executable.read_bytes()
    installed = installed_executable.read_bytes()
    if (
        len(installed) != len(portable)
        or portable.count(TAURI_PORTABLE_BUNDLE_MARKER) != 1
        or installed.count(TAURI_NSIS_BUNDLE_MARKER) != 1
    ):
        return False
    offset = portable.index(TAURI_PORTABLE_BUNDLE_MARKER)
    marker_end = offset + len(TAURI_PORTABLE_BUNDLE_MARKER)
    return bool(
        installed[:offset] == portable[:offset]
        and installed[offset:marker_end] == TAURI_NSIS_BUNDLE_MARKER
        and installed[marker_end:] == portable[marker_end:]
    )


def package_set_fingerprint(packages: Sequence[Mapping[str, Any]]) -> str:
    lines: list[str] = []
    for expected_kind, package in zip(("installed", "portable"), packages):
        if package.get("package_kind") != expected_kind:
            raise ContractError(
                "package identity order must be installed then portable"
            )
        provenance = package.get("provenance")
        if provenance != PACKAGE_PROVENANCE:
            raise ContractError(f"{expected_kind} package provenance is not frozen")
        lines.extend(
            [
                f"package_kind={expected_kind}",
                f"resolved_path={package.get('resolved_path')}",
                f"size={package.get('size')}",
                f"sha256={package.get('sha256')}",
                f"product_version={package.get('product_version')}",
                f"file_version={package.get('file_version')}",
                f"provenance.evidence_kind={provenance['evidence_kind']}",
                f"provenance.file_identity_source={provenance['file_identity_source']}",
                f"provenance.version_identity_source={provenance['version_identity_source']}",
            ]
        )
    return hashlib.sha256("\n".join(lines).encode("utf-8")).hexdigest()


def _validate_package_identities(
    value: Any,
    *,
    installed_executable: Path,
    portable_executable: Path,
) -> tuple[list[dict[str, Any]], str, dict[str, Any]]:
    root = _exact(
        value,
        {
            "schema_version",
            "evidence_kind",
            "packages",
            "package_set_fingerprint",
            "hardware_fingerprint",
        },
        "raw package identities",
    )
    packages = root.get("packages")
    if (
        root.get("schema_version") != 1
        or root.get("evidence_kind") != "general_sem_rank0_package_identities"
        or not isinstance(packages, list)
        or len(packages) != 2
    ):
        raise ContractError("raw package identity evidence is invalid")
    normalized: list[dict[str, Any]] = []
    for expected_kind, expected_path, raw in zip(
        ("installed", "portable"),
        (installed_executable, portable_executable),
        packages,
    ):
        row = _exact(
            raw,
            {
                "package_kind",
                "resolved_path",
                "size",
                "sha256",
                "product_version",
                "file_version",
                "provenance",
            },
            f"{expected_kind} package identity",
        )
        resolved_text = row.get("resolved_path")
        if not isinstance(resolved_text, str) or not Path(resolved_text).is_absolute():
            raise ContractError(f"{expected_kind} package path is not absolute")
        unresolved = Path(resolved_text)
        if unresolved.is_symlink():
            raise ContractError(f"{expected_kind} package path must not be a symlink")
        resolved = unresolved.resolve()
        if resolved != expected_path.resolve() or not resolved.is_file():
            raise ContractError(
                f"{expected_kind} package path differs from the executable prerequisite"
            )
        if (
            row.get("package_kind") != expected_kind
            or row.get("size") != resolved.stat().st_size
            or row.get("sha256") != _sha256_file(resolved)
            or not isinstance(row.get("sha256"), str)
            or not SHA256.fullmatch(row["sha256"])
            or not isinstance(row.get("product_version"), str)
            or not row["product_version"]
            or not isinstance(row.get("file_version"), str)
            or not row["file_version"]
            or row.get("provenance") != PACKAGE_PROVENANCE
        ):
            raise ContractError(
                f"{expected_kind} package identity does not match its current bytes/version authority"
            )
        normalized.append(dict(row))
    if (
        normalized[0]["resolved_path"].casefold()
        == normalized[1]["resolved_path"].casefold()
    ):
        raise ContractError("installed and portable package paths must be distinct")
    fingerprint = package_set_fingerprint(normalized)
    if root.get("package_set_fingerprint") != fingerprint:
        raise ContractError("raw package-set fingerprint does not reproduce")
    if not _is_exact_tauri_nsis_bundle_variant(
        installed_executable, portable_executable
    ):
        raise ContractError(
            "installed executable is not the exact Tauri NSIS marker variant of the portable build"
        )
    hardware = root.get("hardware_fingerprint")
    expected_hardware_fields = {
        "os",
        "architecture",
        "cpu",
        "physical_cores",
        "logical_cores",
        "memory_bytes",
    }
    if not isinstance(hardware, Mapping) or set(hardware) != expected_hardware_fields:
        raise ContractError("raw package hardware fingerprint fields are not exact")
    physical = hardware.get("physical_cores")
    logical = hardware.get("logical_cores")
    memory = hardware.get("memory_bytes")
    if (
        hardware.get("os") != "windows_11"
        or hardware.get("architecture") != "x86_64"
        or not isinstance(hardware.get("cpu"), str)
        or not hardware["cpu"].strip()
        or not isinstance(physical, int)
        or isinstance(physical, bool)
        or physical < 6
        or not isinstance(logical, int)
        or isinstance(logical, bool)
        or logical < physical
        or not isinstance(memory, int)
        or isinstance(memory, bool)
        or memory < 16 * 1024**3
    ):
        raise ContractError(
            "raw package hardware does not satisfy standard_windows_6c16g"
        )
    return normalized, fingerprint, dict(hardware)


def _relative(path: Path, repository_root: Path) -> str:
    resolved = path.resolve()
    try:
        relative = resolved.relative_to(repository_root.resolve())
    except ValueError as error:
        raise ContractError(
            f"evidence path leaves the repository: {resolved}"
        ) from error
    return relative.as_posix()


def _descriptor(path: Path, kind: str, repository_root: Path) -> dict[str, Any]:
    return {
        "kind": kind,
        "path": _relative(path, repository_root),
        "size": path.stat().st_size,
        "sha256": _sha256_file(path),
    }


def _file_descriptor(
    path: Path, format_id: str, repository_root: Path
) -> dict[str, Any]:
    if not path.is_file() or path.is_symlink() or path.stat().st_size <= 0:
        raise ContractError(
            f"{format_id} export is not a non-empty regular file: {path}"
        )
    return {
        "format": format_id,
        "path": _relative(path, repository_root),
        "size": path.stat().st_size,
        "sha256": _sha256_file(path),
    }


def _capability_reference(reference: Sequence[Any]) -> dict[str, Any]:
    return {
        "registry_schema_version": reference[0],
        "capability_id": reference[1],
        "cell_id": reference[2],
        "capability_version": reference[3],
    }


def _reference_tuple(value: Any, subject: str) -> tuple[int, str, str, str]:
    row = _exact(
        value,
        {"registry_schema_version", "capability_id", "cell_id", "capability_version"},
        subject,
    )
    result = (
        row.get("registry_schema_version"),
        row.get("capability_id"),
        row.get("cell_id"),
        row.get("capability_version"),
    )
    if result[0] != 2 or any(
        not isinstance(item, str) or not item for item in result[1:]
    ):
        raise ContractError(f"{subject} is invalid")
    return result  # type: ignore[return-value]


def _load_project_document(path: Path) -> Mapping[str, Any]:
    if not path.is_file() or path.is_symlink() or path.stat().st_size <= 0:
        raise ContractError(f"schema-6 project archive is unavailable: {path}")
    try:
        if path.read_bytes()[:4] == b"PK\x03\x04":
            with zipfile.ZipFile(path, "r") as archive:
                if archive.testzip() is not None:
                    raise ContractError(
                        "schema-6 project ZIP contains a corrupt member"
                    )
                names = archive.namelist()
                if names.count("project.json") != 1:
                    raise ContractError(
                        "schema-6 project ZIP must contain exactly one project.json"
                    )
                value = json.loads(
                    archive.read("project.json"),
                    parse_constant=lambda token: (_ for _ in ()).throw(
                        ContractError(f"non-finite JSON: {token}")
                    ),
                )
        else:
            value = load_json(path)
    except (OSError, UnicodeError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        raise ContractError(str(error)) from error
    if not isinstance(value, Mapping) or value.get("schema_version") != 6:
        raise ContractError(
            "General SEM acceptance requires a strict schema-6 project document"
        )
    if value.get("sem_generation") != "general_sem_v1":
        raise ContractError("project archive is not marked general_sem_v1")
    return value


def _canonical_attachment(
    project: Mapping[str, Any],
    *,
    run_id: str,
    document_id: str,
    variant: Mapping[str, Any],
) -> tuple[Mapping[str, Any], str, Mapping[str, Any]]:
    attachments = project.get("canonical_result_documents")
    if not isinstance(attachments, list) or not attachments:
        raise ContractError("schema-6 project contains no canonical result attachments")
    matches: list[Mapping[str, Any]] = []
    for attachment in attachments:
        if not isinstance(attachment, Mapping):
            continue
        document = attachment.get("canonical_document")
        if not isinstance(document, Mapping):
            continue
        provenance = document.get("provenance")
        observed_run = attachment.get("run_id") or (
            provenance.get("run_id") if isinstance(provenance, Mapping) else None
        )
        observed_document = attachment.get("document_id") or document.get("document_id")
        if observed_run == run_id and observed_document == document_id:
            matches.append(attachment)
    if len(matches) != 1:
        raise ContractError(
            "schema-6 archive does not contain exactly one matching canonical result"
        )
    attachment = matches[0]
    document = attachment["canonical_document"]
    provenance = document.get("provenance")
    if not isinstance(provenance, Mapping):
        raise ContractError("canonical result provenance is missing")
    authority = variant_canonical_authority(variant)
    if (
        _reference_tuple(
            provenance.get("capability_cell"), "canonical primary capability cell"
        )
        != authority["primary"]
    ):
        raise ContractError(
            "canonical result does not reconcile to the exact point-primary Registry cell"
        )
    if provenance.get("method_version") != authority["method_version"]:
        raise ContractError("canonical result outer method_version is invalid")
    inventory = document.get("capability_cells")
    if not isinstance(inventory, list):
        raise ContractError("canonical result capability inventory is missing")
    inventory_cells = [
        _reference_tuple(cell, f"canonical capability_cells[{index}]")
        for index, cell in enumerate(inventory)
    ]
    if inventory_cells.count(authority["primary"]) != 1:
        raise ContractError(
            "canonical result omits or duplicates its point-primary cell"
        )
    results = document.get("general_sem_results")
    receipt = results.get("inference_receipt") if isinstance(results, Mapping) else None
    supplemental = authority["supplemental"]
    if supplemental is None:
        if receipt is not None:
            raise ContractError(
                "point result cannot claim a supplemental inference receipt"
            )
    else:
        if inventory_cells.count(supplemental) != 1:
            raise ContractError(
                "bootstrap canonical inventory omits or duplicates its supplemental cell"
            )
        if (
            not isinstance(receipt, Mapping)
            or _reference_tuple(
                receipt.get("capability_cell"),
                "canonical inference receipt capability cell",
            )
            != supplemental
            or receipt.get("method_version") != authority["method_version"]
        ):
            raise ContractError(
                "bootstrap inference receipt does not bind its exact supplemental cell/method"
            )
    canonical_bytes = json.dumps(
        document,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    digest = hashlib.sha256(canonical_bytes).hexdigest()
    recorded_digest = attachment.get("canonical_document_sha256")
    if recorded_digest != digest or not SHA256.fullmatch(digest):
        raise ContractError(
            "canonical result bytes do not reproduce the attachment digest"
        )
    return document, digest, authority


def _raw_identity(
    value: Mapping[str, Any], package: str, variant: str, kind: str, subject: str
) -> None:
    if value.get("schema_version") != 1 or value.get("evidence_kind") != kind:
        raise ContractError(f"{subject} evidence identity is invalid")
    if value.get("package_kind") != package or value.get("variant_id") != variant:
        raise ContractError(f"{subject} package/variant identity is invalid")
    if value.get("passed") is not True:
        raise ContractError(f"{subject} did not pass")


def _normalize_offline_observation(
    value: Any, *, phase: str, scale_percent: int
) -> dict[str, Any]:
    row = _exact(
        value,
        {
            "passed",
            "observedRequestCount",
            "externalRequestCount",
            "origins",
            "externalRequests",
        },
        f"{phase} functional-offline observation",
    )
    observed = row.get("observedRequestCount")
    external = row.get("externalRequestCount")
    origins = row.get("origins")
    if (
        row.get("passed") is not True
        or not isinstance(observed, int)
        or isinstance(observed, bool)
        or observed <= 0
        or external != 0
        or not isinstance(origins, list)
        or not origins
        or len(origins) != len(set(origins))
        or any(origin not in ALLOWED_OFFLINE_ORIGINS for origin in origins)
        or not any(
            origin in {PACKAGED_TAURI_ORIGIN, PACKAGED_TAURI_IPC_ORIGIN}
            for origin in origins
        )
        or row.get("externalRequests") != []
    ):
        raise ContractError(
            f"{phase} did not prove a positive, zero-external-request offline observation"
        )
    return {
        "phase": phase,
        "scale_percent": scale_percent,
        "observed_request_count": observed,
        "external_request_count": 0,
        "origins": origins,
        "external_requests": [],
        "passed": True,
    }


def _cancel_archive_identity(value: Any, subject: str) -> dict[str, Any]:
    row = _exact(
        value,
        {
            "schema_version",
            "evidence_kind",
            "archive_path",
            "byte_length",
            "sha256",
            "project_schema_version",
            "sem_generation",
            "canonical_result_attachment_count",
        },
        subject,
    )
    if (
        row.get("schema_version") != 1
        or row.get("evidence_kind") != "general_sem_rank0_schema6_archive_identity"
        or row.get("project_schema_version") != 6
        or row.get("sem_generation") != "general_sem_v1"
        or not isinstance(row.get("byte_length"), int)
        or isinstance(row.get("byte_length"), bool)
        or row["byte_length"] <= 0
        or not isinstance(row.get("sha256"), str)
        or not SHA256.fullmatch(row["sha256"])
        or not isinstance(row.get("canonical_result_attachment_count"), int)
        or isinstance(row.get("canonical_result_attachment_count"), bool)
        or row["canonical_result_attachment_count"] < 0
    ):
        raise ContractError(f"{subject} is invalid")
    return {
        "byte_length": row["byte_length"],
        "sha256": row["sha256"],
        "canonical_result_attachment_count": row["canonical_result_attachment_count"],
    }


def _normalize_cancellation(value: Any, required: bool) -> dict[str, Any] | None:
    if not required:
        if value is not None:
            raise ContractError(
                "optional acceptance cannot claim cancellation evidence"
            )
        return None
    row = _exact(
        value,
        {
            "terminalLatencySeconds",
            "terminalState",
            "jobCompletedBeforeCancel",
            "noPartialVisibleResult",
            "noPartialCommittedResult",
            "archiveUnchanged",
            "exactSameSettingsRetry",
            "visibleResultCountBefore",
            "visibleResultCountAfter",
            "committedResultActionCount",
            "archiveBefore",
            "archiveAfter",
            "settingsBefore",
            "settingsRetry",
        },
        "raw cancellation",
    )
    latency = row.get("terminalLatencySeconds")
    if (
        not isinstance(latency, (int, float))
        or isinstance(latency, bool)
        or latency < 0
        or latency > 1
        or row.get("terminalState") != "cancelled"
        or row.get("jobCompletedBeforeCancel") is not False
        or any(
            row.get(field) is not True
            for field in (
                "noPartialVisibleResult",
                "noPartialCommittedResult",
                "archiveUnchanged",
                "exactSameSettingsRetry",
            )
        )
        or not isinstance(row.get("visibleResultCountBefore"), int)
        or row.get("visibleResultCountAfter") != row.get("visibleResultCountBefore")
        or row.get("committedResultActionCount") != 0
        or not isinstance(row.get("settingsBefore"), Mapping)
        or row.get("settingsRetry") != row.get("settingsBefore")
    ):
        raise ContractError(
            "raw cancellation did not satisfy the exact <=1.0s retry gate"
        )
    before = _cancel_archive_identity(row.get("archiveBefore"), "cancel archive before")
    after = _cancel_archive_identity(row.get("archiveAfter"), "cancel archive after")
    if before != after:
        raise ContractError(
            "cancelled calculation changed archive bytes or result-attachment count"
        )
    return {
        "terminal_latency_seconds": latency,
        "terminal_state": "cancelled",
        "job_completed_before_cancel": False,
        "no_partial_visible_result": True,
        "no_partial_committed_result": True,
        "archive_unchanged": True,
        "exact_same_settings_retry": True,
        "archive_before": before,
        "archive_after": after,
    }


def _normalize_export_cancellation(value: Any, repository_root: Path) -> dict[str, Any]:
    row = _exact(value, {"saveDialog"}, "raw export cancellation")
    save = _exact(
        row.get("saveDialog"),
        {
            "format",
            "destinationPath",
            "nativeDialogCancelled",
            "semanticReadbackCompleted",
            "destinationExistedAfter",
            "noPartialFile",
            "publication",
        },
        "raw Save-dialog export cancellation",
    )
    destination = save.get("destinationPath")
    publication = save.get("publication")
    published_file = (
        publication.get("file") if isinstance(publication, Mapping) else None
    )
    if (
        save.get("format") != "csv"
        or not isinstance(destination, str)
        or not Path(destination).is_absolute()
        or save.get("nativeDialogCancelled") is not True
        or save.get("semanticReadbackCompleted") is not True
        or save.get("destinationExistedAfter") is not False
        or save.get("noPartialFile") is not True
        or not isinstance(publication, Mapping)
        or publication.get("event") != "complete"
        or publication.get("passed") is not True
        or publication.get("mode") != "save-cancel"
        or not isinstance(published_file, Mapping)
        or published_file.get("path") != destination
        or published_file.get("exists") is not False
        or published_file.get("cancelledBeforePublication") is not True
        or Path(destination).exists()
    ):
        raise ContractError(
            "cancelled native export did not prove pre-publication zero-file semantics"
        )
    return {
        "save_dialog_destination_path": _relative(Path(destination), repository_root),
        "save_dialog_cancelled": True,
        "semantic_readback_completed": True,
        "save_dialog_no_partial_file": True,
    }


def _normalize_result(
    raw_root: Path,
    repository_root: Path,
    package: str,
    variant: Mapping[str, Any],
    package_identity: Mapping[str, Any],
) -> dict[str, Any]:
    variant_id = variant["variant_id"]
    evidence_dir = raw_root / package / variant_id
    primary = load_json(evidence_dir / "raw-run-trace.json")
    _raw_identity(
        primary, package, variant_id, "general_sem_rank0_primary_run", "primary run"
    )
    exports_raw = load_json(evidence_dir / "raw-exported-files.json")
    if (
        not isinstance(exports_raw, Mapping)
        or exports_raw.get("schema_version") != 1
        or exports_raw.get("evidence_kind") != "raw_exported_files"
        or exports_raw.get("package_kind") != package
        or exports_raw.get("variant_id") != variant_id
    ):
        raise ContractError("raw exported-files evidence identity is invalid")
    cleanup_raw = load_json(evidence_dir / "raw-process-cleanup.json")
    _raw_identity(
        cleanup_raw,
        package,
        variant_id,
        "general_sem_rank0_process_cleanup",
        "process cleanup",
    )
    reopen_raw = []
    for scale in SCALES:
        value = load_json(evidence_dir / f"raw-reopen-{scale}.json")
        _raw_identity(
            value,
            package,
            variant_id,
            "general_sem_rank0_fresh_reopen",
            f"reopen {scale}",
        )
        if value.get("scalePercent") != scale:
            raise ContractError(f"reopen {scale} scale identity is invalid")
        reopen_raw.append(value)

    identity = primary.get("identity")
    if not isinstance(identity, Mapping):
        raise ContractError("primary run canonical identity is missing")
    run_id = identity.get("runId")
    document_id = identity.get("documentId")
    if (
        not isinstance(run_id, str)
        or not run_id
        or not isinstance(document_id, str)
        or not document_id
    ):
        raise ContractError("primary run/document identity is incomplete")
    reference = variant["reference"]
    reference_object = _capability_reference(reference)
    cancellation = _normalize_cancellation(primary.get("cancellation"), True)
    export_cancellation = _normalize_export_cancellation(
        primary.get("exportCancellation"), repository_root
    )
    project_path = evidence_dir / "rank0-general-sem.qpls"
    project = _load_project_document(project_path)
    canonical_document, canonical_digest, canonical_authority = _canonical_attachment(
        project,
        run_id=run_id,
        document_id=document_id,
        variant=variant,
    )
    project_digest = _sha256_file(project_path)

    export_rows = exports_raw.get("files")
    if not isinstance(export_rows, list) or len(export_rows) != len(EXPORT_FORMATS):
        raise ContractError("raw export evidence must contain exactly six files")
    normalized_exports = []
    for index, expected_format in enumerate(EXPORT_FORMATS):
        row = export_rows[index]
        if not isinstance(row, Mapping) or row.get("format") != expected_format:
            raise ContractError("raw export format/order is invalid")
        raw_path = row.get("path")
        if not isinstance(raw_path, str) or not Path(raw_path).is_absolute():
            raise ContractError(
                "raw export paths must be absolute package-driver paths"
            )
        path_value = Path(raw_path).resolve()
        descriptor = _file_descriptor(path_value, expected_format, repository_root)
        if (
            row.get("size") != descriptor["size"]
            or row.get("sha256") != descriptor["sha256"]
        ):
            raise ContractError(
                f"raw {expected_format} export descriptor does not match its bytes"
            )
        try:
            readback = semantic_readback(
                path_value, expected_format, canonical_document
            )
        except ValueError as error:
            raise ContractError(
                f"{expected_format} final-file semantic readback failed: {error}"
            ) from error
        normalized_exports.append({**descriptor, "semantic_readback": readback})
    if (
        exports_raw.get("run_id") != run_id
        or exports_raw.get("document_id") != document_id
    ):
        raise ContractError(
            "six-format export evidence is not bound to the exact canonical run"
        )

    cleanup_sessions_raw = cleanup_raw.get("sessions")
    if not isinstance(cleanup_sessions_raw, list) or len(cleanup_sessions_raw) != 5:
        raise ContractError(
            "process cleanup must contain one primary and four reopen sessions"
        )
    cleanup_sessions = []
    expected_session_ids = [
        "primary",
        "scale_100",
        "scale_125",
        "scale_150",
        "scale_200",
    ]
    for raw, expected_id in zip(cleanup_sessions_raw, expected_session_ids):
        if not isinstance(raw, Mapping) or raw.get("session_id") != expected_id:
            raise ContractError("process cleanup session identity/order is invalid")
        normalized = {
            "session_id": expected_id,
            "phase": raw.get("phase"),
            "scale_percent": raw.get("scale_percent"),
            "launched_pid": raw.get("launched_pid"),
            "launched_executable_path": raw.get("launched_executable_path"),
            "launched_executable_size": raw.get("launched_executable_size"),
            "launched_executable_sha256": raw.get("launched_executable_sha256"),
            "graceful_exit_confirmed": raw.get("graceful_exit_confirmed"),
            "forced_termination": raw.get("forced_termination"),
            "lingering_pids": raw.get("lingering_pids"),
            "cdp_endpoint_closed": raw.get("cdp_endpoint_closed"),
            "passed": raw.get("passed"),
        }
        if (
            normalized["passed"] is not True
            or normalized["graceful_exit_confirmed"] is not True
            or normalized["forced_termination"] is not False
            or normalized["lingering_pids"] != []
            or normalized["cdp_endpoint_closed"] is not True
            or not isinstance(normalized["launched_pid"], int)
            or normalized["launched_executable_path"]
            != package_identity["resolved_path"]
            or normalized["launched_executable_size"] != package_identity["size"]
            or normalized["launched_executable_sha256"] != package_identity["sha256"]
        ):
            raise ContractError(f"process cleanup session {expected_id} is not clean")
        cleanup_sessions.append(normalized)
    pids = [row["launched_pid"] for row in cleanup_sessions]
    if len(pids) != len(set(pids)):
        raise ContractError("fresh-process reopen sessions reused a process identity")
    if primary.get("process_id") != pids[0]:
        raise ContractError(
            "primary browser trace is not bound to the supervised process"
        )

    cells = []
    reopen_sessions = []
    offline_observations = [
        _normalize_offline_observation(
            primary.get("offline"), phase="execute", scale_percent=100
        )
    ]
    for index, (scale, raw) in enumerate(zip(SCALES, reopen_raw), start=1):
        raw_identity = raw.get("identity")
        if not isinstance(raw_identity, Mapping):
            raise ContractError(f"reopen {scale} canonical identity is missing")
        if (
            raw_identity.get("runId") != run_id
            or raw_identity.get("documentId") != document_id
        ):
            raise ContractError(f"reopen {scale} restored a different canonical run")
        raw_archive = raw.get("projectArchive")
        if (
            not isinstance(raw_archive, Mapping)
            or raw_archive.get("sha256") != project_digest
        ):
            raise ContractError(
                f"reopen {scale} project digest differs from the final archive"
            )
        if (
            raw.get("freshProcessReopen") is not True
            or raw.get("closeProject") is not True
        ):
            raise ContractError(
                f"reopen {scale} did not prove fresh open and explicit close"
            )
        raw_cells = raw.get("cells")
        if not isinstance(raw_cells, list) or len(raw_cells) != len(VIEWPORTS):
            raise ContractError(f"reopen {scale} viewport matrix is incomplete")
        for raw_cell, viewport in zip(raw_cells, VIEWPORTS):
            if not isinstance(raw_cell, Mapping) or raw_cell.get("passed") is not True:
                raise ContractError(f"reopen {scale}/{viewport} failed")
            snapshot = raw_cell.get("snapshot")
            keyboard = raw_cell.get("keyboard")
            if not isinstance(snapshot, Mapping) or not isinstance(keyboard, Mapping):
                raise ContractError(
                    f"reopen {scale}/{viewport} a11y evidence is missing"
                )
            cell = {
                "scale_percent": scale,
                "viewport": viewport,
                "origin": snapshot.get("origin"),
                "tauri_runtime": snapshot.get("tauriRuntime"),
                "surface": snapshot.get("surface"),
                "device_pixel_ratio": snapshot.get("devicePixelRatio"),
                "actual_client_width": snapshot.get("innerWidth"),
                "actual_client_height": snapshot.get("innerHeight"),
                "no_horizontal_overflow": bool(
                    snapshot.get("documentNoHorizontalOverflow")
                    and snapshot.get("appNoHorizontalOverflow")
                ),
                "table_count": snapshot.get("tableCount"),
                "accessible_table_count": snapshot.get("accessibleTableCount"),
                "chart_count": snapshot.get("chartCount"),
                "accessible_chart_count": snapshot.get("accessibleChartCount"),
                "keyboard_distinct_targets": keyboard.get("distinctTargets"),
                "keyboard_reached_interactive_control": keyboard.get(
                    "reachedInteractiveControl"
                ),
                "passed": True,
            }
            if (
                cell["origin"] != PACKAGED_TAURI_ORIGIN
                or cell["tauri_runtime"] is not True
                or cell["surface"] != "model"
                or cell["actual_client_width"] != int(viewport.split("x")[0])
                or cell["actual_client_height"] != int(viewport.split("x")[1])
                or cell["no_horizontal_overflow"] is not True
                or not isinstance(cell["table_count"], int)
                or cell["table_count"] <= 0
                or cell["accessible_table_count"] != cell["table_count"]
                or not isinstance(cell["chart_count"], int)
                or cell["chart_count"] <= 0
                or cell["accessible_chart_count"] != cell["chart_count"]
                or not isinstance(cell["keyboard_distinct_targets"], int)
                or cell["keyboard_distinct_targets"] < 4
                or cell["keyboard_reached_interactive_control"] is not True
            ):
                raise ContractError(
                    f"reopen {scale}/{viewport} normalized a11y contract failed"
                )
            cells.append(cell)
        offline_observations.append(
            _normalize_offline_observation(
                raw.get("offline"), phase="reopen", scale_percent=scale
            )
        )
        if raw.get("process_id") != pids[index]:
            raise ContractError(
                f"reopen {scale} browser trace is not bound to the supervised process"
            )
        reopen_sessions.append(
            {
                "scale_percent": scale,
                "process_id": pids[index],
                "run_id": run_id,
                "document_id": document_id,
                "project_archive_sha256": project_digest,
                "closed": True,
                "passed": True,
            }
        )
    raw_steps = primary.get("steps")
    if not isinstance(raw_steps, Mapping):
        raise ContractError("primary run step evidence is missing")
    derived = {
        "fresh_process_reopen": True,
        "keyboard_navigation": True,
        "accessible_table_and_chart": True,
        "viewport_scaling": True,
        "process_cleanup": True,
    }
    steps: dict[str, bool] = {}
    for check_id in variant["required_checks"]:
        value = derived.get(check_id, raw_steps.get(check_id))
        if value is not True:
            raise ContractError(
                f"{package}/{variant_id} did not prove required step {check_id}"
            )
        steps[check_id] = True

    project_descriptor = {
        "path": _relative(project_path, repository_root),
        "size": project_path.stat().st_size,
        "sha256": project_digest,
    }
    temporary_or_partial_files = sorted(
        _relative(path, repository_root)
        for path in evidence_dir.rglob("*")
        if path.is_file()
        and (
            path.name.lower().endswith((".tmp", ".partial", ".part", ".crdownload"))
            or path.name.startswith("~$")
            or path.name.endswith("~")
        )
    )
    orphan_process_ids = sorted(
        {
            process_id
            for session in cleanup_sessions
            for process_id in session["lingering_pids"]
        }
    )
    if temporary_or_partial_files or orphan_process_ids:
        raise ContractError(
            "packaged workflow left temporary/partial files or orphan processes"
        )
    final_documents: list[tuple[str, Path, dict[str, Any]]] = [
        (
            "run_trace",
            evidence_dir / "run-trace.json",
            {
                "schema_version": 1,
                "evidence_kind": "general_sem_rank0_run_trace",
                "package_kind": package,
                "variant_id": variant_id,
                "capability_reference": reference_object,
                "offline": True,
                "offline_observations": offline_observations,
                "steps": steps,
                "run_id": run_id,
                "document_id": document_id,
                "project_archive": project_descriptor,
                "cancellation_observation": cancellation,
                "export_cancellation_observation": export_cancellation,
            },
        ),
        (
            "canonical_result",
            evidence_dir / "canonical-result.json",
            {
                "schema_version": 1,
                "evidence_kind": "general_sem_rank0_canonical_result",
                "package_kind": package,
                "variant_id": variant_id,
                "capability_reference": reference_object,
                "primary_capability_reference": _capability_reference(
                    canonical_authority["primary"]
                ),
                "supplemental_capability_reference": (
                    _capability_reference(canonical_authority["supplemental"])
                    if canonical_authority["supplemental"] is not None
                    else None
                ),
                "method_version": canonical_authority["method_version"],
                "document_id": document_id,
                "run_id": run_id,
                "canonical_document_sha256": canonical_digest,
                "canonical_document": canonical_document,
            },
        ),
        (
            "exported_files_manifest",
            evidence_dir / "exported-files-manifest.json",
            {
                "schema_version": 1,
                "evidence_kind": "general_sem_rank0_exported_files_manifest",
                "package_kind": package,
                "variant_id": variant_id,
                "run_id": run_id,
                "document_id": document_id,
                "files": normalized_exports,
            },
        ),
        (
            "accessibility_snapshot",
            evidence_dir / "accessibility-snapshot.json",
            {
                "schema_version": 1,
                "evidence_kind": "general_sem_rank0_accessibility_snapshot",
                "package_kind": package,
                "variant_id": variant_id,
                "scales": list(SCALES),
                "viewports": list(VIEWPORTS),
                "cells": cells,
                "keyboard_navigation": True,
                "accessible_table_and_chart": True,
                "passed": True,
            },
        ),
        (
            "process_cleanup_trace",
            evidence_dir / "process-cleanup-trace.json",
            {
                "schema_version": 1,
                "evidence_kind": "general_sem_rank0_process_cleanup_trace",
                "package_kind": package,
                "variant_id": variant_id,
                "sessions": cleanup_sessions,
                "orphan_process_ids": orphan_process_ids,
                "temporary_or_partial_files": temporary_or_partial_files,
                "passed": True,
            },
        ),
        (
            "close_reopen_trace",
            evidence_dir / "close-reopen-trace.json",
            {
                "schema_version": 1,
                "evidence_kind": "general_sem_rank0_close_reopen_trace",
                "package_kind": package,
                "variant_id": variant_id,
                "project_archive_sha256": project_digest,
                "run_id": run_id,
                "document_id": document_id,
                "primary_pid": pids[0],
                "reopen_sessions": reopen_sessions,
                "passed": True,
            },
        ),
    ]
    if tuple(kind for kind, _, _ in final_documents) != REQUIRED_ARTIFACT_KINDS:
        raise AssertionError("evidence role order drifted")
    for _, path, document in final_documents:
        _write_new_json(path, document)
    return {
        "package_kind": package,
        "variant_id": variant_id,
        "capability_reference": reference_object,
        "offline": True,
        "fresh_process_reopen": True,
        "checks": steps,
        "artifacts": [
            _descriptor(path, kind, repository_root)
            for kind, path, _ in final_documents
        ],
    }


def compose_report(
    *,
    raw_root: Path,
    output: Path,
    build_fingerprint: str,
    package_set_fingerprint_value: str,
    package_identities: Sequence[Mapping[str, Any]],
    hardware_fingerprint: Mapping[str, Any],
    variant_id: str | None = None,
    contract_path: Path = DEFAULT_CONTRACT,
    registry_path: Path = DEFAULT_REGISTRY,
    repository_root: Path = ROOT,
    require_standard: bool = False,
) -> dict[str, Any]:
    if not SHA256.fullmatch(build_fingerprint):
        raise ContractError("build_fingerprint must be an exact executable SHA-256")
    if (
        not SHA256.fullmatch(package_set_fingerprint_value)
        or package_set_fingerprint(package_identities) != package_set_fingerprint_value
    ):
        raise ContractError(
            "package_set_fingerprint must reproduce exact installed+portable identities"
        )
    package_hashes = [row.get("sha256") for row in package_identities]
    if package_hashes[1] != build_fingerprint:
        raise ContractError(
            "build_fingerprint must equal the portable pre-package executable SHA-256"
        )
    if output.exists():
        raise ContractError(f"report output must be new: {output}")
    _relative(raw_root, repository_root)
    _relative(output, repository_root)
    contract = load_json(contract_path)
    context = validate_contract(contract, load_json(registry_path))
    package_by_kind = {row["package_kind"]: row for row in package_identities}
    if set(package_by_kind) != set(context["packages"]):
        raise ContractError("package identity set does not match acceptance contract")
    selected_variants = [
        variant
        for variant in context["variants"]
        if variant_id is None or variant["variant_id"] == variant_id
    ]
    if len(selected_variants) != (
        len(context["variants"]) if variant_id is None else 1
    ):
        raise ContractError("requested packaged variant is unavailable")
    results = [
        _normalize_result(
            raw_root,
            repository_root,
            package,
            variant,
            package_by_kind[package],
        )
        for package in context["packages"]
        for variant in selected_variants
    ]
    report = {
        "schema_version": 1,
        "report_kind": "quickpls_general_sem_rank0_packaged_acceptance",
        "contract_id": context["contract_id"],
        "contract_version": context["contract_version"],
        "contract_sha256": context["contract_sha256"],
        "build_fingerprint": build_fingerprint,
        "package_set_fingerprint": package_set_fingerprint_value,
        "package_identities": list(package_identities),
        "hardware_fingerprint": dict(hardware_fingerprint),
        "source_receipt": unified_rank0_source_receipt(repository_root),
        "qualification_contracts": qualification_contract_authorities(
            context, repository_root
        ),
        "generated_at_utc": datetime.now(timezone.utc)
        .isoformat()
        .replace("+00:00", "Z"),
        "results": results,
    }
    if variant_id is None:
        validate_report(
            report, context, repository_root, require_standard=require_standard
        )
    else:
        target = selected_variants[0]
        validate_cell_report(
            report,
            context,
            repository_root,
            capability_reference={
                "registry_schema_version": target["reference"][0],
                "capability_id": target["reference"][1],
                "cell_id": target["reference"][2],
                "capability_version": target["reference"][3],
            },
            require_standard=require_standard,
        )
    _write_new_json(output, report)
    return report


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--build-fingerprint", required=True)
    parser.add_argument("--package-identities", type=Path, required=True)
    parser.add_argument("--installed-executable", type=Path, required=True)
    parser.add_argument("--portable-executable", type=Path, required=True)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    parser.add_argument("--repository-root", type=Path, default=ROOT)
    parser.add_argument("--require-standard", action="store_true")
    parser.add_argument("--variant-id")
    args = parser.parse_args(argv)
    try:
        for kind, executable in (
            ("installed", args.installed_executable),
            ("portable", args.portable_executable),
        ):
            if not executable.is_file() or executable.suffix.lower() != ".exe":
                raise ContractError(
                    f"{kind} executable prerequisite is missing: {executable}"
                )
        (
            package_identities,
            reproduced_package_set_fingerprint,
            hardware_fingerprint,
        ) = _validate_package_identities(
            load_json(args.package_identities.resolve()),
            installed_executable=args.installed_executable.resolve(),
            portable_executable=args.portable_executable.resolve(),
        )
        if package_identities[1]["sha256"] != args.build_fingerprint:
            raise ContractError(
                "command build fingerprint differs from the portable pre-package executable bytes"
            )
        report = compose_report(
            raw_root=args.raw_root.resolve(),
            output=args.output.resolve(),
            build_fingerprint=args.build_fingerprint,
            package_set_fingerprint_value=reproduced_package_set_fingerprint,
            package_identities=package_identities,
            hardware_fingerprint=hardware_fingerprint,
            variant_id=args.variant_id,
            contract_path=args.contract.resolve(),
            registry_path=args.registry.resolve(),
            repository_root=args.repository_root.resolve(),
            require_standard=args.require_standard,
        )
        result = {
            "passed": True,
            "output": str(args.output.resolve()),
            "result_count": len(report["results"]),
            "build_fingerprint": report["build_fingerprint"],
            "package_set_fingerprint": report["package_set_fingerprint"],
        }
    except (ContractError, OSError, UnicodeError, json.JSONDecodeError) as error:
        result = {"passed": False, "errors": [str(error)]}
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
