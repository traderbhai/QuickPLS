#!/usr/bin/env python3
"""Pack and restore identity-bound runtime evidence for clean-checkout CI.

Method factory identity reports intentionally bind executable, archive, export,
and screenshot bytes that are too large or too ephemeral to keep as ordinary
Git source.  This tool makes those bytes portable without trusting the bundle
itself: the checked-in identity reports remain authoritative, every payload is
matched by repository-relative path, size, and SHA-256, and restoration refuses
overwrites, links, traversal, unexpected entries, and source-commit drift.

The bundle is evidence transport only.  It never promotes a method or authorizes
commercial, beta, stable, or competitor claims.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
IDENTITY_PATTERN = "validation/results/method_factory/**/*.identity.json"
MANIFEST_ENTRY = "quickpls-release-evidence-manifest.json"
PAYLOAD_PREFIX = "payload/"
SHA256 = re.compile(r"^[0-9a-f]{64}$")
COMMIT = re.compile(r"^[0-9a-f]{40}$")
RUNTIME_PREFIXES = ("target/release/", "validation/results/")
MAX_ARTIFACT_COUNT = 2_000
MAX_ARTIFACT_SIZE = 256 * 1024 * 1024
MAX_TOTAL_SIZE = 2 * 1024 * 1024 * 1024
MAX_MANIFEST_SIZE = 8 * 1024 * 1024


class EvidenceBundleError(ValueError):
    """Raised when evidence transport cannot be proven safe and exact."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceBundleError(message)


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        _require(key not in value, f"duplicate JSON key is not allowed: {key}")
        value[key] = item
    return value


def _reject_constant(value: str) -> None:
    raise EvidenceBundleError(f"non-finite JSON number is not allowed: {value}")


def _load_json_bytes(payload: bytes, label: str) -> dict[str, Any]:
    try:
        document = json.loads(
            payload.decode("utf-8"),
            object_pairs_hook=_strict_object,
            parse_constant=_reject_constant,
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        raise EvidenceBundleError(f"{label} is not strict UTF-8 JSON: {error}") from error
    _require(isinstance(document, dict), f"{label} must be a JSON object")
    return document


def _load_json(path: Path, label: str) -> dict[str, Any]:
    try:
        return _load_json_bytes(path.read_bytes(), label)
    except OSError as error:
        raise EvidenceBundleError(f"cannot read {label}: {error}") from error


def _relative_path(value: object, label: str) -> str:
    _require(isinstance(value, str) and bool(value), f"{label} must be a non-empty string")
    _require("\\" not in value and "\x00" not in value, f"{label} must use a safe POSIX path")
    path = PurePosixPath(value)
    _require(not path.is_absolute(), f"{label} must be repository-relative")
    _require(path.parts and all(part not in {"", ".", ".."} for part in path.parts), f"{label} contains an unsafe component")
    normalized = path.as_posix()
    _require(normalized == value, f"{label} is not canonical")
    return normalized


def _runtime_path(value: object, label: str) -> str:
    path = _relative_path(value, label)
    _require(path.startswith(RUNTIME_PREFIXES), f"{label} is outside the frozen runtime evidence roots")
    return path


def _descriptor(path: Path, root: Path) -> dict[str, Any]:
    resolved = path.resolve()
    _require(resolved.is_relative_to(root.resolve()), f"evidence file escapes the repository: {path}")
    _require(path.is_file() and not path.is_symlink(), f"evidence file is missing or linked: {path}")
    size = path.stat().st_size
    _require(0 < size <= MAX_ARTIFACT_SIZE, f"evidence file has an invalid size: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return {
        "path": resolved.relative_to(root.resolve()).as_posix(),
        "size": size,
        "sha256": digest.hexdigest(),
    }


def _validated_declared_descriptor(value: object, label: str) -> dict[str, Any]:
    _require(isinstance(value, dict), f"{label} must be an object")
    path = _relative_path(value.get("path"), f"{label}.path")
    size = value.get("size")
    digest = value.get("sha256")
    _require(isinstance(size, int) and not isinstance(size, bool) and 0 < size <= MAX_ARTIFACT_SIZE, f"{label}.size is invalid")
    _require(isinstance(digest, str) and bool(SHA256.fullmatch(digest)), f"{label}.sha256 is invalid")
    return {"path": path, "size": size, "sha256": digest}


def _git(root: Path, *arguments: str) -> str:
    completed = subprocess.run(
        ["git", *arguments],
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if completed.returncode != 0:
        raise EvidenceBundleError(f"git {' '.join(arguments)} failed: {completed.stderr.strip()}")
    return completed.stdout


def git_commit(root: Path) -> str:
    value = _git(root, "rev-parse", "HEAD").strip().lower()
    _require(bool(COMMIT.fullmatch(value)), "Git HEAD is not a full commit identity")
    return value


def git_tracked_paths(root: Path) -> set[str]:
    return {item.replace("\\", "/") for item in _git(root, "ls-files").splitlines() if item}


def require_clean_source(root: Path) -> None:
    changed = _git(root, "status", "--porcelain=v1", "--untracked-files=all").splitlines()
    # Runtime evidence and the destination bundle may be ignored; any visible
    # change means the checked-in identity contract is not a clean source state.
    _require(not changed, "evidence packing requires a clean Git source checkout")


def collect_runtime_requirements(
    root: Path,
    *,
    tracked_paths: set[str] | None = None,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    root = root.resolve()
    tracked = tracked_paths if tracked_paths is not None else git_tracked_paths(root)
    report_paths = sorted(root.glob(IDENTITY_PATTERN))
    _require(report_paths, "no method-factory identity reports were found")
    reports: list[dict[str, Any]] = []
    artifacts: dict[str, dict[str, Any]] = {}
    for report_path in report_paths:
        report_descriptor = _descriptor(report_path, root)
        _require(report_descriptor["path"] in tracked, f"identity report is not tracked: {report_descriptor['path']}")
        report = _load_json(report_path, f"identity report {report_descriptor['path']}")
        _require(report.get("report_kind") == "quickpls_method_factory_identity_report", f"identity report kind is invalid: {report_descriptor['path']}")
        _require(report.get("passed") is True, f"identity report did not pass: {report_descriptor['path']}")
        source_artifacts = report.get("source_artifacts")
        _require(isinstance(source_artifacts, list) and source_artifacts, f"identity report has no source artifacts: {report_descriptor['path']}")
        reports.append(report_descriptor)
        for index, raw in enumerate(source_artifacts):
            declared = _validated_declared_descriptor(raw, f"{report_descriptor['path']}.source_artifacts[{index}]")
            if declared["path"] in tracked:
                actual = _descriptor(root / Path(*PurePosixPath(declared["path"]).parts), root)
                _require(
                    actual == declared,
                    f"tracked source differs from its identity report: {declared['path']}",
                )
                continue
            runtime_path = _runtime_path(declared["path"], f"runtime source in {report_descriptor['path']}")
            declared["path"] = runtime_path
            previous = artifacts.get(runtime_path)
            _require(previous is None or previous == declared, f"identity reports conflict for runtime artifact: {runtime_path}")
            artifacts[runtime_path] = declared
    values = [artifacts[path] for path in sorted(artifacts)]
    _require(values, "identity reports do not require any transported runtime evidence")
    _require(len(values) <= MAX_ARTIFACT_COUNT, "runtime evidence entry count exceeds the limit")
    _require(sum(item["size"] for item in values) <= MAX_TOTAL_SIZE, "runtime evidence total exceeds the limit")
    return reports, values


def _canonical_json(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def pack_bundle(
    bundle_path: Path,
    *,
    root: Path = ROOT,
    source_commit: str | None = None,
    expected_commit: str | None = None,
    tracked_paths: set[str] | None = None,
    generated_at: str | None = None,
    require_clean: bool = True,
) -> dict[str, Any]:
    root = root.resolve()
    bundle_path = bundle_path.resolve()
    _require(not bundle_path.exists() and not bundle_path.is_symlink(), f"refusing to overwrite evidence bundle: {bundle_path}")
    if expected_commit is not None:
        expected_commit = expected_commit.lower()
        _require(bool(COMMIT.fullmatch(expected_commit)), "expected_commit must be a full lowercase Git commit")
    if source_commit is None:
        if require_clean:
            require_clean_source(root)
        source_commit = git_commit(root)
    source_commit = source_commit.lower()
    _require(bool(COMMIT.fullmatch(source_commit)), "source_commit must be a full lowercase Git commit")
    if expected_commit is not None:
        _require(source_commit == expected_commit, "Git HEAD does not match the explicitly expected source commit")
    reports, requirements = collect_runtime_requirements(root, tracked_paths=tracked_paths)
    verified: list[dict[str, Any]] = []
    for requirement in requirements:
        actual = _descriptor(root / Path(*PurePosixPath(requirement["path"]).parts), root)
        _require(actual == requirement, f"runtime artifact differs from its identity reports: {requirement['path']}")
        verified.append(actual)
    timestamp = generated_at or datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
    try:
        parsed = datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
    except ValueError as error:
        raise EvidenceBundleError("generated_at must be ISO-8601") from error
    _require(parsed.tzinfo is not None, "generated_at must include an offset")
    manifest = {
        "schema_version": 1,
        "document_type": "quickpls_release_evidence_transport",
        "source_commit": source_commit,
        "generated_at_utc": timestamp,
        "identity_reports": reports,
        "artifacts": verified,
        "limits": {
            "artifact_count": MAX_ARTIFACT_COUNT,
            "artifact_size": MAX_ARTIFACT_SIZE,
            "total_size": MAX_TOTAL_SIZE,
        },
        "trust": {
            "identity_reports_authoritative": True,
            "transport_verified": True,
            "method_state_derived": False,
            "commercial_gate_satisfied": False,
            "competitor_claims_authorized": False,
        },
    }
    bundle_path.parent.mkdir(parents=True, exist_ok=True)
    created = False
    try:
        with zipfile.ZipFile(bundle_path, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            created = True
            archive.writestr(MANIFEST_ENTRY, _canonical_json(manifest))
            for artifact in verified:
                source = root / Path(*PurePosixPath(artifact["path"]).parts)
                archive.write(source, arcname=f"{PAYLOAD_PREFIX}{artifact['path']}")
        return {
            "passed": True,
            "operation": "pack",
            "source_commit": source_commit,
            "artifact_count": len(verified),
            "artifact_bytes": sum(item["size"] for item in verified),
            "bundle": _descriptor(bundle_path, bundle_path.parent),
            "commercial_gate_satisfied": False,
            "competitor_claims_authorized": False,
        }
    except BaseException:
        if created:
            bundle_path.unlink(missing_ok=True)
        raise


def _zip_entries(archive: zipfile.ZipFile) -> dict[str, zipfile.ZipInfo]:
    entries: dict[str, zipfile.ZipInfo] = {}
    for info in archive.infolist():
        _require(len(entries) <= MAX_ARTIFACT_COUNT, "evidence ZIP entry count exceeds the limit")
        name = _relative_path(info.filename, "ZIP entry")
        _require(name not in entries, f"duplicate ZIP entry: {name}")
        _require(not info.is_dir(), f"directory ZIP entries are not allowed: {name}")
        mode = (info.external_attr >> 16) & 0xFFFF
        _require(not stat.S_ISLNK(mode), f"linked ZIP entries are not allowed: {name}")
        entries[name] = info
    return entries


def _validate_manifest(
    manifest: dict[str, Any],
    *,
    current_reports: list[dict[str, Any]],
    current_requirements: list[dict[str, Any]],
    expected_commit: str,
) -> None:
    _require(
        set(manifest)
        == {"schema_version", "document_type", "source_commit", "generated_at_utc", "identity_reports", "artifacts", "limits", "trust"},
        "evidence manifest keys differ from the frozen schema",
    )
    _require(manifest["schema_version"] == 1, "evidence manifest schema_version must be 1")
    _require(manifest["document_type"] == "quickpls_release_evidence_transport", "evidence manifest document_type is invalid")
    _require(manifest["source_commit"] == expected_commit, "evidence bundle source commit does not match the checkout")
    _require(manifest["identity_reports"] == current_reports, "evidence bundle is not bound to the current identity reports")
    _require(manifest["artifacts"] == current_requirements, "evidence bundle runtime inventory differs from current identity requirements")
    _require(
        manifest["limits"]
        == {"artifact_count": MAX_ARTIFACT_COUNT, "artifact_size": MAX_ARTIFACT_SIZE, "total_size": MAX_TOTAL_SIZE},
        "evidence manifest limits differ from the frozen limits",
    )
    _require(
        manifest["trust"]
        == {
            "identity_reports_authoritative": True,
            "transport_verified": True,
            "method_state_derived": False,
            "commercial_gate_satisfied": False,
            "competitor_claims_authorized": False,
        },
        "evidence transport trust boundary is invalid",
    )


def _safe_destination(root: Path, relative: str) -> Path:
    target = root / Path(*PurePosixPath(relative).parts)
    resolved_root = root.resolve()
    _require(target.parent.resolve().is_relative_to(resolved_root), f"evidence destination escapes the repository: {relative}")
    cursor = root
    for component in PurePosixPath(relative).parts[:-1]:
        cursor = cursor / component
        if cursor.exists() or cursor.is_symlink():
            _require(cursor.is_dir() and not cursor.is_symlink(), f"evidence destination ancestor is not a real directory: {relative}")
        else:
            cursor.mkdir()
    _require(not target.exists() and not target.is_symlink(), f"refusing to overwrite restored evidence: {relative}")
    return target


def restore_bundle(
    bundle_path: Path,
    *,
    root: Path = ROOT,
    expected_commit: str | None = None,
    tracked_paths: set[str] | None = None,
    verify_only: bool = False,
) -> dict[str, Any]:
    root = root.resolve()
    bundle_path = bundle_path.resolve()
    _require(bundle_path.is_file() and not bundle_path.is_symlink(), f"evidence bundle is missing or linked: {bundle_path}")
    commit = (expected_commit or git_commit(root)).lower()
    _require(bool(COMMIT.fullmatch(commit)), "expected commit must be a full lowercase Git commit")
    reports, requirements = collect_runtime_requirements(root, tracked_paths=tracked_paths)
    created: list[Path] = []
    staging = Path(tempfile.mkdtemp(prefix=".quickpls-evidence-", dir=root))
    try:
        _require(bundle_path.stat().st_size <= MAX_TOTAL_SIZE + MAX_MANIFEST_SIZE, "evidence bundle exceeds the transport size limit")
        with zipfile.ZipFile(bundle_path, "r") as archive:
            entries = _zip_entries(archive)
            _require(MANIFEST_ENTRY in entries, "evidence bundle manifest is missing")
            _require(entries[MANIFEST_ENTRY].file_size <= MAX_MANIFEST_SIZE, "evidence bundle manifest is too large")
            manifest = _load_json_bytes(archive.read(entries[MANIFEST_ENTRY]), "evidence bundle manifest")
            _validate_manifest(
                manifest,
                current_reports=reports,
                current_requirements=requirements,
                expected_commit=commit,
            )
            expected_entries = {MANIFEST_ENTRY, *(f"{PAYLOAD_PREFIX}{item['path']}" for item in requirements)}
            _require(set(entries) == expected_entries, "evidence bundle contains missing or unexpected entries")
            staged: list[tuple[dict[str, Any], Path]] = []
            total = 0
            for index, artifact in enumerate(requirements):
                name = f"{PAYLOAD_PREFIX}{artifact['path']}"
                info = entries[name]
                _require(info.file_size == artifact["size"], f"ZIP size differs from identity for {artifact['path']}")
                total += info.file_size
                _require(total <= MAX_TOTAL_SIZE, "evidence ZIP payload exceeds the total size limit")
                temporary = staging / f"artifact-{index:04d}"
                digest = hashlib.sha256()
                size = 0
                with archive.open(info, "r") as source, temporary.open("xb") as destination:
                    while True:
                        block = source.read(1024 * 1024)
                        if not block:
                            break
                        size += len(block)
                        _require(size <= artifact["size"], f"ZIP payload expanded beyond the declared size: {artifact['path']}")
                        digest.update(block)
                        destination.write(block)
                _require(size == artifact["size"] and digest.hexdigest() == artifact["sha256"], f"ZIP payload hash differs from identity: {artifact['path']}")
                staged.append((artifact, temporary))
        if not verify_only:
            for artifact, temporary in staged:
                destination = _safe_destination(root, artifact["path"])
                with temporary.open("rb") as source, destination.open("xb") as target:
                    created.append(destination)
                    shutil.copyfileobj(source, target, length=1024 * 1024)
                    target.flush()
                    os.fsync(target.fileno())
                _require(_descriptor(destination, root) == artifact, f"restored evidence failed final verification: {artifact['path']}")
        return {
            "passed": True,
            "operation": "verify" if verify_only else "restore",
            "source_commit": commit,
            "artifact_count": len(requirements),
            "artifact_bytes": sum(item["size"] for item in requirements),
            "commercial_gate_satisfied": False,
            "competitor_claims_authorized": False,
        }
    except BaseException:
        for path in reversed(created):
            path.unlink(missing_ok=True)
        raise
    finally:
        shutil.rmtree(staging, ignore_errors=True)


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="operation", required=True)
    pack = subparsers.add_parser("pack", help="create an identity-bound evidence ZIP")
    pack.add_argument("--bundle", type=Path, required=True)
    pack.add_argument("--expected-commit", required=True)
    restore = subparsers.add_parser("restore", help="verify and restore an evidence ZIP")
    restore.add_argument("--bundle", type=Path, required=True)
    restore.add_argument("--verify-only", action="store_true")
    args = parser.parse_args(list(argv) if argv is not None else None)
    try:
        if args.operation == "pack":
            report = pack_bundle(args.bundle, expected_commit=args.expected_commit)
        else:
            report = restore_bundle(args.bundle, verify_only=args.verify_only)
    except (EvidenceBundleError, OSError, zipfile.BadZipFile) as error:
        print(json.dumps({"passed": False, "commercial_gate_satisfied": False, "errors": [str(error)]}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
