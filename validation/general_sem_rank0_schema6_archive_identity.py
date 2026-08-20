#!/usr/bin/env python3
"""Emit a strict byte and result-attachment identity for one schema-6 archive."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import zipfile
from pathlib import Path
from typing import Any, Mapping, Sequence


class ArchiveIdentityError(ValueError):
    pass


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ArchiveIdentityError(f"duplicate JSON key in project.json: {key}")
        value[key] = item
    return value


def inspect_archive(path: Path) -> dict[str, Any]:
    if not path.is_absolute():
        raise ArchiveIdentityError("schema-6 archive path must be absolute")
    if not path.is_file() or path.is_symlink():
        raise ArchiveIdentityError("schema-6 archive must be a regular non-link file")
    stat_before = path.stat()
    payload = path.read_bytes()
    stat_after = path.stat()
    if (
        stat_before.st_size != stat_after.st_size
        or stat_before.st_mtime_ns != stat_after.st_mtime_ns
        or stat_after.st_size != len(payload)
        or not payload
    ):
        raise ArchiveIdentityError("schema-6 archive changed while its identity was read")
    try:
        with zipfile.ZipFile(io.BytesIO(payload), "r") as archive:
            names = archive.namelist()
            if names.count("project.json") != 1 or len(names) != len(set(names)):
                raise ArchiveIdentityError(
                    "schema-6 archive requires one project.json and no duplicate members"
                )
            corrupt = archive.testzip()
            if corrupt is not None:
                raise ArchiveIdentityError(f"schema-6 archive has a corrupt member: {corrupt}")
            document = json.loads(
                archive.read("project.json"),
                object_pairs_hook=_strict_object,
                parse_constant=lambda token: (_ for _ in ()).throw(
                    ArchiveIdentityError(f"non-finite project.json value: {token}")
                ),
            )
    except (OSError, UnicodeError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        raise ArchiveIdentityError(str(error)) from error
    if not isinstance(document, Mapping) or document.get("schema_version") != 6:
        raise ArchiveIdentityError("project.json is not strict schema version 6")
    if document.get("sem_generation") != "general_sem_v1":
        raise ArchiveIdentityError("project.json is not general_sem_v1")
    attachments = document.get("canonical_result_documents", [])
    if not isinstance(attachments, list):
        raise ArchiveIdentityError("schema-6 canonical_result_documents is not an array")
    return {
        "schema_version": 1,
        "evidence_kind": "general_sem_rank0_schema6_archive_identity",
        "archive_path": str(path.resolve()),
        "byte_length": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "project_schema_version": 6,
        "sem_generation": "general_sem_v1",
        "canonical_result_attachment_count": len(attachments),
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        result: Mapping[str, Any] = inspect_archive(args.archive)
        exit_code = 0
    except (ArchiveIdentityError, OSError) as error:
        result = {"passed": False, "errors": [str(error)]}
        exit_code = 1
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
