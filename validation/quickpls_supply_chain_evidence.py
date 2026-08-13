#!/usr/bin/env python3
"""Generate exact-candidate QuickPLS SBOM, license, and provenance evidence.

This command is intentionally useful for unsigned internal previews without
allowing those bytes to satisfy the signed beta/stable release gate.  It reads
the preserved-artifact report, rehashes every artifact, derives dependency
metadata from the committed JavaScript and Rust lock graphs, and writes a
CycloneDX SBOM, a reviewable license inventory, provenance, and one binding
manifest beside the candidate artifacts.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import platform
import subprocess
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ARTIFACT_REPORT = ROOT / "validation" / "results" / "release_artifacts.json"
DEFAULT_REPORT = ROOT / "validation" / "results" / "quickpls_supply_chain_evidence.json"
SHA256 = __import__("re").compile(r"^[0-9a-fA-F]{64}$")
ALLOWED_UNSIGNED_CHANNELS = {"internal", "unsigned-preview"}
REQUIRED_ARTIFACT_ROLES = {"portable", "cli", "setup", "checksums"}


class EvidenceError(ValueError):
    """Raised when exact-candidate supply-chain evidence cannot be trusted."""


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise EvidenceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _strict_float(token: str) -> float:
    value = float(token)
    if not math.isfinite(value):
        raise EvidenceError(f"non-finite JSON number: {token}")
    return value


def load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8-sig") as handle:
            return json.load(
                handle,
                object_pairs_hook=_strict_object,
                parse_constant=lambda token: (_ for _ in ()).throw(
                    EvidenceError(f"non-finite JSON number: {token}")
                ),
                parse_float=_strict_float,
            )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise EvidenceError(f"cannot read JSON {path}: {error}") from error


def canonical_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def file_descriptor(path: Path, root: Path = ROOT) -> dict[str, Any]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            size += len(block)
            digest.update(block)
    try:
        relative = path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError as error:
        raise EvidenceError(f"evidence path is outside the repository: {path}") from error
    return {"path": relative, "size": size, "sha256": digest.hexdigest()}


def safe_repository_file(root: Path, value: Any, label: str) -> Path:
    if not isinstance(value, str) or not value or "\\" in value:
        raise EvidenceError(f"{label} must be a non-empty POSIX repository path")
    relative = PurePosixPath(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise EvidenceError(f"{label} must not escape the repository")
    path = (root / Path(*relative.parts)).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise EvidenceError(f"{label} resolves outside the repository") from error
    if not path.is_file() or path.is_symlink():
        raise EvidenceError(f"{label} is missing or is not a regular file: {value}")
    return path


def validate_artifact_report(path: Path, root: Path = ROOT) -> dict[str, Any]:
    report = load_json(path)
    if not isinstance(report, dict) or report.get("schema_version") != 2:
        raise EvidenceError("artifact report must use unsigned-preview schema version 2")
    if report.get("passed") is not True:
        raise EvidenceError("artifact report did not pass")
    channel = report.get("release_channel")
    if channel not in ALLOWED_UNSIGNED_CHANNELS:
        raise EvidenceError(
            "this generator currently accepts only internal or unsigned-preview candidates; "
            "signed beta/stable evidence must use the signed-candidate factory"
        )
    trust = report.get("trust")
    if not isinstance(trust, dict) or trust.get("competitor_claims_authorized") is not False:
        raise EvidenceError("unsigned artifact trust boundary is missing")
    if trust.get("authenticode_verification_performed") is not False:
        raise EvidenceError("unsigned report must not claim Authenticode verification")
    artifacts = report.get("artifacts")
    if not isinstance(artifacts, list) or len(artifacts) != len(REQUIRED_ARTIFACT_ROLES):
        raise EvidenceError("artifact report must contain the exact four preserved artifact roles")
    roles = [row.get("role") for row in artifacts if isinstance(row, dict)]
    if set(roles) != REQUIRED_ARTIFACT_ROLES or len(roles) != len(set(roles)):
        raise EvidenceError("artifact roles differ from portable, CLI, setup, and checksums")
    verified: list[dict[str, Any]] = []
    for row in artifacts:
        if not isinstance(row, dict) or row.get("copy_verified") is not True:
            raise EvidenceError("artifact copy verification is missing")
        artifact = safe_repository_file(root, row.get("path"), f"{row.get('role')} artifact")
        actual = file_descriptor(artifact, root)
        reported_hash = row.get("sha256")
        if not isinstance(reported_hash, str) or not SHA256.fullmatch(reported_hash):
            raise EvidenceError(f"{row.get('role')} artifact hash is invalid")
        if row.get("bytes") != actual["size"] or reported_hash.lower() != actual["sha256"]:
            raise EvidenceError(f"{row.get('role')} artifact bytes differ from the report")
        verified.append({"role": row["role"], **actual})
    return {"report": report, "artifacts": sorted(verified, key=lambda row: row["role"])}


def npm_components(package_lock: dict[str, Any]) -> tuple[list[dict[str, Any]], dict[str, list[str]]]:
    packages = package_lock.get("packages")
    if not isinstance(packages, dict) or "" not in packages:
        raise EvidenceError("package-lock packages table is missing the root package")
    components: list[dict[str, Any]] = []
    refs_by_path: dict[str, str] = {}
    for package_path, package in sorted(packages.items()):
        if not package_path or not isinstance(package, dict):
            continue
        name = package.get("name")
        if not isinstance(name, str) or not name:
            name = package_path.rsplit("node_modules/", 1)[-1]
        version = package.get("version")
        license_expression = package.get("license")
        if not isinstance(version, str) or not version:
            raise EvidenceError(f"npm component {package_path!r} has no version")
        if not isinstance(license_expression, str) or not license_expression:
            raise EvidenceError(f"npm component {name}@{version} has no license expression")
        component_ref = f"pkg:npm/{name.replace('@', '%40')}@{version}?path={package_path}"
        refs_by_path[package_path] = component_ref
        components.append(
            {
                "type": "library",
                "bom-ref": component_ref,
                "name": name,
                "version": version,
                "purl": f"pkg:npm/{name.replace('@', '%40')}@{version}",
                "licenses": [{"expression": license_expression}],
                "properties": [
                    {"name": "quickpls:ecosystem", "value": "npm"},
                    {"name": "quickpls:lock_path", "value": package_path},
                    {
                        "name": "quickpls:distribution_scope",
                        "value": "build" if package.get("dev") is True else "production",
                    },
                ],
            }
        )
    dependency_map: dict[str, list[str]] = {}
    for package_path, component_ref in refs_by_path.items():
        package = packages[package_path]
        dependencies = package.get("dependencies", {})
        resolved: list[str] = []
        if isinstance(dependencies, dict):
            for dependency_name in sorted(dependencies):
                candidates = []
                cursor = package_path
                while cursor:
                    candidates.append(f"{cursor}/node_modules/{dependency_name}")
                    cursor = cursor.rsplit("/node_modules/", 1)[0] if "/node_modules/" in cursor else ""
                candidates.append(f"node_modules/{dependency_name}")
                target = next((refs_by_path[item] for item in candidates if item in refs_by_path), None)
                if target is not None:
                    resolved.append(target)
        dependency_map[component_ref] = sorted(set(resolved))
    return components, dependency_map


def cargo_components(metadata: dict[str, Any]) -> tuple[list[dict[str, Any]], dict[str, list[str]]]:
    packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolve, dict):
        raise EvidenceError("cargo metadata is missing packages or the resolved graph")
    by_id: dict[str, str] = {}
    components: list[dict[str, Any]] = []
    for package in packages:
        if not isinstance(package, dict):
            raise EvidenceError("cargo metadata package is not an object")
        package_id = package.get("id")
        name = package.get("name")
        version = package.get("version")
        license_expression = package.get("license")
        if not all(isinstance(value, str) and value for value in (package_id, name, version)):
            raise EvidenceError("cargo component identity is incomplete")
        if not isinstance(license_expression, str) or not license_expression:
            raise EvidenceError(f"cargo component {name}@{version} has no license expression")
        component_ref = f"urn:quickpls:cargo:{sha256_bytes(package_id.encode('utf-8'))}"
        by_id[package_id] = component_ref
        components.append(
            {
                "type": "library" if name != "quickpls-desktop" else "application",
                "bom-ref": component_ref,
                "name": name,
                "version": version,
                "purl": f"pkg:cargo/{name}@{version}",
                "licenses": [{"expression": license_expression}],
                "properties": [
                    {"name": "quickpls:ecosystem", "value": "cargo"},
                    {"name": "quickpls:package_id", "value": package_id},
                    {"name": "quickpls:distribution_scope", "value": "resolved-build-graph"},
                ],
            }
        )
    dependencies: dict[str, list[str]] = {reference: [] for reference in by_id.values()}
    nodes = resolve.get("nodes")
    if not isinstance(nodes, list):
        raise EvidenceError("cargo resolved graph nodes are missing")
    for node in nodes:
        if not isinstance(node, dict) or node.get("id") not in by_id:
            raise EvidenceError("cargo resolved graph references an unknown package")
        targets: list[str] = []
        for dependency in node.get("dependencies", []):
            if dependency not in by_id:
                raise EvidenceError("cargo dependency references an unknown package")
            targets.append(by_id[dependency])
        dependencies[by_id[node["id"]]] = sorted(set(targets))
    return components, dependencies


def run_cargo_metadata(root: Path) -> dict[str, Any]:
    completed = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if completed.returncode != 0 or completed.stderr.strip():
        raise EvidenceError(
            f"cargo metadata failed or wrote stderr (exit={completed.returncode}): "
            f"{completed.stderr.strip()}"
        )
    try:
        value = json.loads(
            completed.stdout,
            object_pairs_hook=_strict_object,
            parse_float=_strict_float,
        )
    except (json.JSONDecodeError, EvidenceError) as error:
        raise EvidenceError(f"cargo metadata output is invalid: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError("cargo metadata output must be an object")
    return value


def git_identity(root: Path) -> dict[str, Any]:
    def run(*arguments: str) -> str:
        completed = subprocess.run(
            ["git", *arguments], cwd=root, check=False, capture_output=True, text=True, encoding="utf-8"
        )
        if completed.returncode != 0:
            raise EvidenceError(f"git {' '.join(arguments)} failed")
        return completed.stdout.strip()

    changed = sorted(line[3:].replace("\\", "/") for line in run("status", "--porcelain=v1").splitlines() if len(line) >= 4)
    return {
        "commit": run("rev-parse", "HEAD"),
        "branch": run("branch", "--show-current"),
        "clean": not changed,
        "changed_repository_paths": changed,
    }


def tool_version(command: list[str]) -> str:
    completed = subprocess.run(command, check=False, capture_output=True, text=True, encoding="utf-8")
    if completed.returncode != 0:
        return "unavailable"
    return (completed.stdout or completed.stderr).strip().splitlines()[0]


def build_documents(
    *,
    artifact_report: dict[str, Any],
    artifact_descriptor: dict[str, Any],
    artifacts: list[dict[str, Any]],
    package_lock: dict[str, Any],
    cargo_metadata: dict[str, Any],
    git: dict[str, Any],
    generated_at: str,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    version = artifact_report.get("version")
    channel = artifact_report.get("release_channel")
    if not isinstance(version, str) or not version:
        raise EvidenceError("artifact report version is missing")
    npm, npm_dependencies = npm_components(package_lock)
    cargo, cargo_dependencies = cargo_components(cargo_metadata)
    components = sorted(npm + cargo, key=lambda row: row["bom-ref"])
    if len({row["bom-ref"] for row in components}) != len(components):
        raise EvidenceError("component references are not unique")
    candidate_seed = f"{version}|{channel}|{artifact_report.get('timestamp_utc')}|{artifact_descriptor['sha256']}"
    candidate_id = str(uuid.uuid5(uuid.NAMESPACE_URL, candidate_seed))
    application_ref = f"pkg:generic/quickpls@{version}?candidate={candidate_id}"
    dependencies = {**npm_dependencies, **cargo_dependencies}
    dependencies[application_ref] = sorted(row["bom-ref"] for row in components)
    sbom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": f"urn:uuid:{candidate_id}",
        "version": 1,
        "metadata": {
            "timestamp": generated_at,
            "tools": {"components": [{"type": "application", "name": "quickpls_supply_chain_evidence", "version": "1"}]},
            "component": {
                "type": "application",
                "bom-ref": application_ref,
                "name": "QuickPLS",
                "version": version,
                "properties": [
                    {"name": "quickpls:candidate_id", "value": candidate_id},
                    {"name": "quickpls:release_channel", "value": channel},
                    {"name": "quickpls:artifact_report_sha256", "value": artifact_descriptor["sha256"]},
                    {"name": "quickpls:competitor_claims_authorized", "value": "false"},
                ],
            },
        },
        "components": components,
        "dependencies": [
            {"ref": reference, "dependsOn": targets}
            for reference, targets in sorted(dependencies.items())
        ],
    }
    licenses = {
        "schema_version": 1,
        "candidate_id": candidate_id,
        "version": version,
        "release_channel": channel,
        "generated_at_utc": generated_at,
        "legal_review_complete": False,
        "components": [
            {
                "bom_ref": row["bom-ref"],
                "name": row["name"],
                "version": row["version"],
                "ecosystem": next(
                    property_["value"]
                    for property_ in row["properties"]
                    if property_["name"] == "quickpls:ecosystem"
                ),
                "license_expression": row["licenses"][0]["expression"],
            }
            for row in components
        ],
        "review_boundary": "Machine-derived inventory only; qualified legal review remains mandatory.",
    }
    provenance = {
        "schema_version": 1,
        "candidate_id": candidate_id,
        "version": version,
        "release_channel": channel,
        "generated_at_utc": generated_at,
        "source": git,
        "build": {
            "artifact_report": artifact_descriptor,
            "artifacts": artifacts,
            "toolchain": {
                "python": platform.python_version(),
                "node": tool_version(["node", "--version"]),
                "npm": tool_version(["npm", "--version"]),
                "rustc": tool_version(["rustc", "--version"]),
                "cargo": tool_version(["cargo", "--version"]),
            },
        },
        "trust": {
            "authenticode_verified": False,
            "signed_candidate": False,
            "clean_source_checkout": git["clean"],
            "commercial_gate_eligible": False,
            "competitor_claims_authorized": False,
        },
        "limitations": [
            "Unsigned preview provenance does not establish publisher authenticity.",
            "A dirty source checkout is recorded and cannot be promoted as a protected release build.",
            "Signed beta/stable provenance must be generated after live Authenticode verification.",
        ],
    }
    return sbom, licenses, provenance


def generate_supply_chain_evidence(
    *,
    root: Path = ROOT,
    artifact_report_path: Path = DEFAULT_ARTIFACT_REPORT,
    report_path: Path = DEFAULT_REPORT,
    output_directory: Path | None = None,
    cargo_metadata_path: Path | None = None,
    generator_path: Path | None = None,
    generated_at: str | None = None,
) -> dict[str, Any]:
    root = root.resolve()
    artifact_report_path = artifact_report_path.resolve()
    report_path = report_path.resolve()
    validated = validate_artifact_report(artifact_report_path, root)
    artifact_report = validated["report"]
    artifact_descriptor = file_descriptor(artifact_report_path, root)
    package_lock_path = root / "package-lock.json"
    package_lock = load_json(package_lock_path)
    if not isinstance(package_lock, dict):
        raise EvidenceError("package-lock root must be an object")
    if cargo_metadata_path is None:
        cargo_metadata = run_cargo_metadata(root)
        cargo_input_descriptor = file_descriptor(root / "Cargo.lock", root)
    else:
        cargo_metadata = load_json(cargo_metadata_path)
        if not isinstance(cargo_metadata, dict):
            raise EvidenceError("cargo metadata fixture must be an object")
        cargo_input_descriptor = file_descriptor(cargo_metadata_path.resolve(), root)
    source = git_identity(root)
    timestamp = generated_at or datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
    try:
        parsed_time = datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
        if parsed_time.tzinfo is None:
            raise ValueError
    except ValueError as error:
        raise EvidenceError("generated_at must be an offset-aware ISO-8601 timestamp") from error
    sbom, licenses, provenance = build_documents(
        artifact_report=artifact_report,
        artifact_descriptor=artifact_descriptor,
        artifacts=validated["artifacts"],
        package_lock=package_lock,
        cargo_metadata=cargo_metadata,
        git=source,
        generated_at=timestamp,
    )
    candidate_id = provenance["candidate_id"]
    output = (output_directory or artifact_report_path.parent).resolve()
    try:
        output.relative_to(root)
    except ValueError as error:
        raise EvidenceError("output directory must remain inside the repository") from error
    output.mkdir(parents=True, exist_ok=True)
    stem = f"QuickPLS_{artifact_report['version']}_{artifact_report['release_channel']}_{candidate_id}"
    documents = {
        "sbom": (output / f"{stem}_sbom.cdx.json", sbom),
        "licenses": (output / f"{stem}_licenses.json", licenses),
        "provenance": (output / f"{stem}_provenance.json", provenance),
    }
    collisions = [path for path, _ in documents.values() if path.exists() or path.is_symlink()]
    if collisions:
        raise EvidenceError(f"refusing to overwrite supply-chain evidence: {collisions}")
    created: list[Path] = []
    try:
        for path, document in documents.values():
            path.write_bytes(canonical_json_bytes(document))
            created.append(path)
        descriptors = {name: file_descriptor(path, root) for name, (path, _) in documents.items()}
        manifest = {
            "schema_version": 1,
            "passed": True,
            "candidate_id": candidate_id,
            "version": artifact_report["version"],
            "release_channel": artifact_report["release_channel"],
            "generated_at_utc": timestamp,
            "inputs": {
                "artifact_report": artifact_descriptor,
                "package_lock": file_descriptor(package_lock_path, root),
                "cargo_graph": cargo_input_descriptor,
                "generator": file_descriptor(
                    (generator_path or Path(__file__)).resolve(), root
                ),
            },
            "outputs": descriptors,
            "component_counts": {
                "total": len(sbom["components"]),
                "npm": sum(
                    any(prop["name"] == "quickpls:ecosystem" and prop["value"] == "npm" for prop in row["properties"])
                    for row in sbom["components"]
                ),
                "cargo": sum(
                    any(prop["name"] == "quickpls:ecosystem" and prop["value"] == "cargo" for prop in row["properties"])
                    for row in sbom["components"]
                ),
            },
            "trust": provenance["trust"],
            "commercial_readiness": {
                "supply_chain_gate_satisfied": False,
                "reason": "Unsigned preview evidence is useful for review but cannot satisfy signed beta/stable provenance.",
            },
        }
        report_path.parent.mkdir(parents=True, exist_ok=True)
        if report_path.exists() or report_path.is_symlink():
            raise EvidenceError(f"refusing to overwrite supply-chain report: {report_path}")
        report_path.write_bytes(canonical_json_bytes(manifest))
        created.append(report_path)
        return manifest
    except BaseException:
        for path in reversed(created):
            path.unlink(missing_ok=True)
        raise


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact-report", type=Path, default=DEFAULT_ARTIFACT_REPORT)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--output-directory", type=Path)
    parser.add_argument("--cargo-metadata", type=Path)
    args = parser.parse_args(argv)
    try:
        report = generate_supply_chain_evidence(
            artifact_report_path=args.artifact_report,
            report_path=args.report,
            output_directory=args.output_directory,
            cargo_metadata_path=args.cargo_metadata,
        )
    except EvidenceError as error:
        print(json.dumps({"passed": False, "commercial_ready": False, "errors": [str(error)]}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
