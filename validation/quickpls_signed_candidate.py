"""Assemble a durable, independently verifiable signed QuickPLS candidate.

The factory is intentionally fail-closed. It requires the repository's one
approved leaf signing identity, already signed/timestamped PE inputs, an exact
clean protected-GitHub-workflow context, and access to the approved certificate
private key for detached CMS signatures. It cannot be used before those real
external prerequisites exist.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import uuid
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    from validation.package_release_artifacts import read_release_channel_contract, read_version_contract
    from validation.quickpls_3_release_readiness import (
        SIGNTOOL_ARGUMENTS,
        SIGNING_IDENTITY_RECORD,
        _candidate_digest_map,
        _candidate_distribution_identity,
        _candidate_payload_identity,
        _normalize_signtool_output,
        _parse_signtool_identity,
        _run_windows_file_identity,
        _validate_build_attestation,
        _validate_live_leaf_and_pe_identity,
        _validate_provenance,
        _validate_sbom,
        _validate_signing_identity_policy,
        _validate_updater_zip,
        _verify_detached_cms,
    )
    from validation.quickpls_supply_chain_evidence import (
        cargo_components,
        git_identity,
        load_json,
        npm_components,
        run_cargo_metadata,
        tool_version,
    )
except ModuleNotFoundError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation.package_release_artifacts import read_release_channel_contract, read_version_contract
    from validation.quickpls_3_release_readiness import (  # type: ignore[no-redef]
        SIGNTOOL_ARGUMENTS,
        SIGNING_IDENTITY_RECORD,
        _candidate_digest_map,
        _candidate_distribution_identity,
        _candidate_payload_identity,
        _normalize_signtool_output,
        _parse_signtool_identity,
        _run_windows_file_identity,
        _validate_build_attestation,
        _validate_live_leaf_and_pe_identity,
        _validate_provenance,
        _validate_sbom,
        _validate_signing_identity_policy,
        _validate_updater_zip,
        _verify_detached_cms,
    )
    from validation.quickpls_supply_chain_evidence import (  # type: ignore[no-redef]
        cargo_components,
        git_identity,
        load_json,
        npm_components,
        run_cargo_metadata,
        tool_version,
    )


ROOT = Path(__file__).resolve().parents[1]
SAFE_LABEL = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$")
PROTECTED_WORKFLOW_SUFFIX = "/.github/workflows/release.yml@refs/heads/main"
SIGNING_POLICY = {
    "record": SIGNING_IDENTITY_RECORD,
    "candidate_binding": "exact_record_sha256_and_identity_id",
    "leaf_verification": "windows_authenticode_subject_and_sha1_thumbprint",
    "caller_supplied_patterns": "prohibited",
}


def fail(message: str) -> None:
    raise SystemExit(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def descriptor(path: Path, root: Path = ROOT) -> dict[str, object]:
    resolved = path.resolve()
    if not resolved.is_relative_to(root.resolve()) or not resolved.is_file() or resolved.is_symlink():
        fail(f"Candidate artifact must be an existing regular repository file: {path}")
    relative = resolved.relative_to(root.resolve())
    if relative.parts and relative.parts[0].casefold() in {"target", "node_modules"}:
        fail(f"Candidate artifact must use durable storage, not {relative.parts[0]}: {path}")
    size = resolved.stat().st_size
    if size <= 0:
        fail(f"Candidate artifact is empty: {path}")
    return {"path": relative.as_posix(), "size": size, "sha256": sha256(resolved)}


def locate_signtool(explicit: str | None = None) -> str:
    if explicit is not None:
        candidate = Path(explicit)
        if candidate.is_file():
            return str(candidate.resolve())
        fail(f"Explicit Windows SignTool path does not exist: {explicit}")
    candidates = [os.environ.get("QUICKPLS_SIGNTOOL_PATH"), shutil.which("signtool.exe"), shutil.which("signtool")]
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return str(Path(candidate).resolve())
    fail("Windows SignTool is required. Set QUICKPLS_SIGNTOOL_PATH or add signtool.exe to PATH.")


def approved_signer(root: Path = ROOT) -> dict[str, object]:
    try:
        return _validate_signing_identity_policy(SIGNING_POLICY, repository_root=root, require_approved=True)
    except ValueError as error:
        fail(str(error))


def parse_aware_timestamp(value: str, label: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        fail(f"{label} must be an ISO-8601 timestamp: {error}")
    if parsed.tzinfo is None:
        fail(f"{label} must include a timezone")
    return parsed


def protected_build_context(
    source_commit: str,
    environment: dict[str, str] | None = None,
    *,
    root: Path = ROOT,
) -> dict[str, str]:
    env = environment or os.environ
    required = {
        "GITHUB_ACTIONS": "true",
        "GITHUB_REF_PROTECTED": "true",
        "GITHUB_REF": "refs/heads/main",
    }
    for key, expected in required.items():
        if env.get(key, "").casefold() != expected:
            fail(f"Signed candidates require {key}={expected} in the protected release workflow")
    workflow_path = root / ".github" / "workflows" / "release.yml"
    if not workflow_path.is_file():
        fail("The reviewed .github/workflows/release.yml protected release workflow is not implemented")
    values: dict[str, str] = {}
    for key in ("GITHUB_WORKFLOW_REF", "GITHUB_RUN_ID", "GITHUB_REPOSITORY", "RUNNER_ENVIRONMENT"):
        value = env.get(key, "").strip()
        if not value:
            fail(f"Signed candidates require protected-workflow context {key}")
        values[key] = value
    expected_workflow_ref = f"{values['GITHUB_REPOSITORY']}{PROTECTED_WORKFLOW_SUFFIX}"
    if values["GITHUB_WORKFLOW_REF"] != expected_workflow_ref:
        fail("GITHUB_WORKFLOW_REF is not the frozen main-branch QuickPLS release workflow")
    github_sha = env.get("GITHUB_SHA", "").strip().lower()
    if github_sha != source_commit:
        fail("GITHUB_SHA does not match the clean source commit")
    return {
        "workflow_id": values["GITHUB_WORKFLOW_REF"].split("@", 1)[0],
        "workflow_run_id": values["GITHUB_RUN_ID"],
        "workflow_ref": values["GITHUB_WORKFLOW_REF"],
        "repository": values["GITHUB_REPOSITORY"],
        "runner_environment": values["RUNNER_ENVIRONMENT"],
        "oidc_subject": f"repo:{values['GITHUB_REPOSITORY']}:ref:refs/heads/main",
    }


def verify_signature(
    signtool: str,
    path: Path,
    *,
    role: str,
    version: str,
    signer: dict[str, object],
) -> dict[str, object]:
    before = sha256(path)
    try:
        completed = subprocess.run(
            [signtool, *SIGNTOOL_ARGUMENTS, str(path.resolve())],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=120,
            check=False,
            shell=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        fail(f"SignTool execution failed for {path}: {error}")
    output = _normalize_signtool_output(completed.stdout, completed.stderr, path)
    if completed.returncode != 0:
        fail(f"SignTool rejected {path} with exit {completed.returncode}: {output}")
    try:
        _publisher_hint, timestamp = _parse_signtool_identity(output, f"signed candidate {path.name}")
        live_identity = _validate_live_leaf_and_pe_identity(
            _run_windows_file_identity(path),
            approved_signer=signer,
            role=role,
            target_release=version,
            label=f"signed candidate {path.name}",
        )
    except ValueError as error:
        fail(str(error))
    after = sha256(path)
    if before != after:
        fail(f"Artifact changed during SignTool verification: {path}")
    return {
        "command": SIGNTOOL_ARGUMENTS,
        "exit_code": completed.returncode,
        "verification_output": output,
        "verification_output_sha256": hashlib.sha256(output.encode()).hexdigest(),
        "timestamp": timestamp,
        "verified_file_sha256": after,
        "signer_identity_id": signer["identity_id"],
        **live_identity,
    }


def sign_detached_cms(payload: Path, signature: Path, *, signer: dict[str, object]) -> dict[str, object]:
    if signature.exists() or signature.is_symlink():
        fail(f"Refusing to overwrite detached signature: {signature}")
    powershell = shutil.which("powershell.exe") or shutil.which("pwsh.exe") or shutil.which("pwsh")
    if not powershell:
        fail("Windows PowerShell is required to create detached CMS signatures")
    script = r"""
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try { Add-Type -AssemblyName System.Security.Cryptography.Pkcs } catch { Add-Type -AssemblyName System.Security }
$thumbprint = $env:QPLS_CMS_SIGNER_THUMBPRINT.Replace(' ', '').ToUpperInvariant()
$cert = Get-ChildItem -LiteralPath ("Cert:\CurrentUser\My\" + $thumbprint)
if ($null -eq $cert -or -not $cert.HasPrivateKey) { throw 'Approved signing certificate private key is unavailable' }
$content = [Security.Cryptography.Pkcs.ContentInfo]::new([IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $env:QPLS_CMS_SIGN_PAYLOAD).Path))
$cms = [Security.Cryptography.Pkcs.SignedCms]::new($content, $true)
$cmsSigner = [Security.Cryptography.Pkcs.CmsSigner]::new($cert)
$cmsSigner.IncludeOption = [Security.Cryptography.X509Certificates.X509IncludeOption]::EndCertOnly
$cms.ComputeSignature($cmsSigner)
[IO.File]::WriteAllBytes($env:QPLS_CMS_SIGN_OUTPUT, $cms.Encode())
""".strip()
    environment = os.environ.copy()
    environment["QPLS_CMS_SIGN_PAYLOAD"] = str(payload.resolve())
    environment["QPLS_CMS_SIGN_OUTPUT"] = str(signature.resolve())
    environment["QPLS_CMS_SIGNER_THUMBPRINT"] = str(signer["leaf_sha1_thumbprint"])
    try:
        completed = subprocess.run(
            [str(Path(powershell).resolve()), "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=120,
            check=False,
            shell=False,
            env=environment,
        )
    except (OSError, subprocess.SubprocessError) as error:
        fail(f"Detached CMS signing failed: {error}")
    if completed.returncode != 0 or not signature.is_file() or signature.stat().st_size == 0:
        fail(f"Detached CMS signing failed: {(completed.stdout + completed.stderr).strip()}")
    try:
        return _verify_detached_cms(payload, signature, approved_signer=signer, label=f"detached signature {signature.name}")
    except ValueError as error:
        fail(str(error))


def write_new_json(path: Path, value: object, *, root: Path = ROOT) -> dict[str, object]:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("x", encoding="utf-8", newline="\n") as handle:
            json.dump(value, handle, indent=2, ensure_ascii=False)
            handle.write("\n")
    except FileExistsError:
        fail(f"Refusing to overwrite candidate evidence: {path}")
    return descriptor(path, root)


def candidate_sbom(
    *,
    version: str,
    identity: str,
    distribution: dict[str, dict[str, object]],
    components: list[dict[str, object]],
    dependency_graph: dict[str, list[str]],
    generated_at: str,
) -> dict[str, object]:
    digests = _candidate_digest_map(distribution)
    application_ref = f"pkg:generic/quickpls@{version}?candidate={identity}"
    dependencies = dict(dependency_graph)
    dependencies[application_ref] = sorted(str(row["bom-ref"]) for row in components)
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": f"urn:uuid:{uuid.uuid5(uuid.NAMESPACE_URL, identity)}",
        "version": 1,
        "metadata": {
            "timestamp": generated_at,
            "tools": {"components": [{"type": "application", "name": "quickpls_signed_candidate", "version": "2"}]},
            "component": {
                "type": "application",
                "bom-ref": application_ref,
                "name": "QuickPLS",
                "version": version,
                "purl": f"pkg:generic/quickpls@{version}",
                "licenses": [{"license": {"name": "Proprietary"}}],
                "properties": [
                    {"name": "quickpls:candidate_id", "value": identity},
                    {"name": "quickpls:target_release", "value": version},
                    {"name": "quickpls:candidate_artifact_digests", "value": json.dumps(digests, sort_keys=True, separators=(",", ":"))},
                ],
            },
        },
        "components": sorted(components, key=lambda row: str(row["bom-ref"])),
        "dependencies": [
            {"ref": reference, "dependsOn": targets}
            for reference, targets in sorted(dependencies.items())
        ],
    }


def build_attestation_document(
    *, version: str, identity: str, distribution: dict[str, dict[str, object]], signer: dict[str, object],
    source_commit: str, context: dict[str, str], build_started_at: str, build_finished_at: str,
    sbom_sha256: str,
    root: Path = ROOT,
) -> dict[str, object]:
    return {
        "schema_version": 1,
        "document_type": "quickpls_protected_build_attestation",
        "target_release": version,
        "candidate_id": identity,
        "candidate_artifact_digests": _candidate_digest_map(distribution),
        "signing_identity_id": signer["identity_id"],
        "sbom_sha256": sbom_sha256,
        "source_commit": source_commit,
        "source_tree_clean": True,
        "protected_build": context,
        "build_id": f"github-actions:{context['repository']}:{context['workflow_run_id']}",
        "builder_identity": f"github-actions:{context['workflow_ref']}:{context['runner_environment']}",
        "build_started_at": build_started_at,
        "build_finished_at": build_finished_at,
        "toolchain": {
            "rustc": tool_version(["rustc", "--version"]),
            "cargo": tool_version(["cargo", "--version"]),
            "node": tool_version(["node", "--version"]),
            "npm": tool_version(["npm", "--version"]),
            "tauri_cli": tool_version(["npm", "exec", "tauri", "--", "--version"]),
        },
        "lockfiles": {"Cargo.lock": sha256(root / "Cargo.lock"), "package-lock.json": sha256(root / "package-lock.json")},
    }


def provenance_document(
    *, version: str, identity: str, distribution: dict[str, dict[str, object]], signer: dict[str, object],
    source_commit: str, context: dict[str, str], build_started_at: str, build_finished_at: str,
    sbom: dict[str, object], attestation: dict[str, object], attestation_signature: dict[str, object],
    root: Path = ROOT,
) -> dict[str, object]:
    digests = _candidate_digest_map(distribution)
    return {
        "_type": "https://in-toto.io/Statement/v1",
        "subject": [{"name": role, "digest": {"sha256": digest}} for role, digest in sorted(digests.items())],
        "predicateType": "https://slsa.dev/provenance/v1",
        "predicate": {
            "buildDefinition": {
                "buildType": "https://quickpls.org/build-types/windows-protected-release/v1",
                "externalParameters": {"target_release": version, "channel": "signed", "candidate_id": identity},
                "internalParameters": {"signing_identity_id": signer["identity_id"]},
                "resolvedDependencies": [
                    {"uri": "git+repository", "digest": {"gitCommit": source_commit}},
                    {"uri": "file:Cargo.lock", "digest": {"sha256": sha256(root / "Cargo.lock")}},
                    {"uri": "file:package-lock.json", "digest": {"sha256": sha256(root / "package-lock.json")}},
                ],
            },
            "runDetails": {
                "builder": {"id": f"github-actions:{context['workflow_ref']}"},
                "metadata": {
                    "invocationId": f"github-actions:{context['repository']}:{context['workflow_run_id']}",
                    "startedOn": build_started_at,
                    "finishedOn": build_finished_at,
                },
                "byproducts": [
                    {"name": "cyclonedx-sbom", "digest": {"sha256": hashlib.sha256((json.dumps(sbom, indent=2, ensure_ascii=False) + "\n").encode()).hexdigest()}},
                    {"name": "protected-build-attestation", "digest": {"sha256": attestation["sha256"]}},
                    {"name": "protected-build-attestation-signature", "digest": {"sha256": attestation_signature["sha256"]}},
                ],
            },
        },
    }


def build_signed_candidate(
    *, channel: str, label: str, desktop: Path, cli: Path, installer: Path,
    minimum_installed_version: str, build_started_at: str, build_finished_at: str, output_dir: Path,
    signtool: str | None = None, root: Path = ROOT,
) -> dict[str, Any]:
    root = root.resolve()
    version, _ = read_version_contract(root)
    channels = read_release_channel_contract(root, expected_version=version)["channels"]
    if channel not in {"beta", "stable"} or channels[channel]["artifact_factory"] != "signed_candidate":
        fail("Signed candidate channel must be beta or stable")
    if not SAFE_LABEL.fullmatch(label):
        fail("Candidate label must use 1-64 letters, digits, dot, underscore, or hyphen")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", minimum_installed_version):
        fail("minimum_installed_version must use semantic-version syntax")
    minimum_core = tuple(int(item) for item in minimum_installed_version.split("-", 1)[0].split("."))
    target_core = tuple(int(item) for item in version.split("-", 1)[0].split("."))
    if minimum_core > target_core:
        fail("minimum_installed_version cannot exceed the target release")
    started = parse_aware_timestamp(build_started_at, "build_started_at")
    finished = parse_aware_timestamp(build_finished_at, "build_finished_at")
    if finished < started:
        fail("build_finished_at must not precede build_started_at")
    source = git_identity(root)
    source_commit = source.get("commit")
    if source.get("clean") is not True:
        fail("Signed beta/stable candidates require a clean source checkout")
    if not isinstance(source_commit, str) or not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        fail("Signed candidate source commit must be lowercase 40-hex Git identity")
    context = protected_build_context(source_commit, root=root)
    signer = approved_signer(root)
    tool = locate_signtool(signtool)
    output_dir = output_dir.resolve()
    if not output_dir.is_relative_to(root):
        fail("Signed candidate output directory must stay inside the repository")
    relative_output = output_dir.relative_to(root)
    if len(relative_output.parts) < 2 or tuple(part.casefold() for part in relative_output.parts[:2]) != ("release", "candidates"):
        fail("Signed candidates must use durable release/candidates storage")
    output_dir.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    prefix = f"QuickPLS_{version}_{channel}_{label}_{timestamp}_x64"
    source_paths = {"desktop": desktop.resolve(), "cli": cli.resolve(), "installer": installer.resolve()}
    for role, path in source_paths.items():
        if not path.is_file():
            fail(f"Missing signed {role}: {path}")

    created: list[Path] = []
    try:
        source_verification = {
            role: verify_signature(tool, path, role=role, version=version, signer=signer)
            for role, path in source_paths.items()
        }
        copied_paths = {
            "desktop": output_dir / f"{prefix}_portable.exe",
            "cli": output_dir / f"{prefix}_cli.exe",
            "installer": output_dir / f"{prefix}_setup.exe",
        }
        for role, destination in copied_paths.items():
            if destination.exists() or destination.is_symlink():
                fail(f"Refusing to overwrite candidate artifact: {destination}")
            shutil.copyfile(source_paths[role], destination)
            created.append(destination)
            if sha256(destination) != source_verification[role]["verified_file_sha256"]:
                fail(f"Signed {role} source/copy hash mismatch")
        copy_verification = {
            role: verify_signature(tool, path, role=role, version=version, signer=signer)
            for role, path in copied_paths.items()
        }
        for role in copied_paths:
            if source_verification[role] != copy_verification[role]:
                fail(f"Signed {role} verification changed after immutable copy")

        artifact_map = {role: descriptor(path, root) for role, path in copied_paths.items()}
        payload_id = _candidate_payload_identity(version, artifact_map)
        channel_manifest_path = output_dir / f"{prefix}_channel_manifest.json"
        channel_manifest = {
            "schema_version": 1,
            "document_type": "quickpls_signed_channel_manifest",
            "channel": channel,
            "target_release": version,
            "payload_id": payload_id,
            "signing_identity_id": signer["identity_id"],
            "minimum_installed_version": minimum_installed_version,
            "allow_downgrade": False,
            "manual_check_default": True,
            "installer": artifact_map["installer"],
            "recovery": {"mode": "offline_full_installer", "full_installer_sha256": artifact_map["installer"]["sha256"]},
        }
        artifact_map["channel_manifest"] = write_new_json(channel_manifest_path, channel_manifest, root=root)
        created.append(channel_manifest_path)
        channel_signature_path = output_dir / f"{prefix}_channel_manifest.p7s"
        channel_signature_verification = sign_detached_cms(channel_manifest_path, channel_signature_path, signer=signer)
        created.append(channel_signature_path)
        artifact_map["channel_manifest_signature"] = descriptor(channel_signature_path, root)

        updater = output_dir / f"{prefix}_updater.zip"
        if updater.exists() or updater.is_symlink():
            fail(f"Refusing to overwrite updater: {updater}")
        with zipfile.ZipFile(updater, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            archive.write(copied_paths["installer"], arcname=f"QuickPLS_{version}_x64-setup.exe")
            archive.write(channel_manifest_path, arcname="quickpls-channel-manifest.json")
            archive.write(channel_signature_path, arcname="quickpls-channel-manifest.p7s")
        created.append(updater)
        artifact_map["updater_bundle"] = descriptor(updater, root)
        distribution = {role: artifact_map[role] for role in sorted({"desktop", "cli", "installer", "updater_bundle", "channel_manifest", "channel_manifest_signature"})}
        identity = _candidate_distribution_identity(version, distribution)

        package_lock = load_json(root / "package-lock.json")
        if not isinstance(package_lock, dict):
            fail("package-lock.json must be an object")
        npm_rows, npm_graph = npm_components(package_lock)
        cargo_rows, cargo_graph = cargo_components(run_cargo_metadata(root))
        generated_at = datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
        sbom = candidate_sbom(
            version=version, identity=identity, distribution=distribution,
            components=[*npm_rows, *cargo_rows], dependency_graph={**npm_graph, **cargo_graph}, generated_at=generated_at,
        )
        sbom_path = output_dir / f"{prefix}_sbom.cdx.json"
        artifact_map["sbom"] = write_new_json(sbom_path, sbom, root=root)
        created.append(sbom_path)
        build_attestation = build_attestation_document(
            version=version, identity=identity, distribution=distribution, signer=signer,
            source_commit=source_commit, context=context, build_started_at=build_started_at, build_finished_at=build_finished_at,
            sbom_sha256=str(artifact_map["sbom"]["sha256"]), root=root,
        )
        attestation_path = output_dir / f"{prefix}_build_attestation.json"
        artifact_map["build_attestation"] = write_new_json(attestation_path, build_attestation, root=root)
        created.append(attestation_path)
        attestation_signature_path = output_dir / f"{prefix}_build_attestation.p7s"
        attestation_signature_verification = sign_detached_cms(attestation_path, attestation_signature_path, signer=signer)
        created.append(attestation_signature_path)
        artifact_map["build_attestation_signature"] = descriptor(attestation_signature_path, root)
        provenance = provenance_document(
            version=version, identity=identity, distribution=distribution, signer=signer,
            source_commit=source_commit, context=context, build_started_at=build_started_at, build_finished_at=build_finished_at,
            sbom=sbom, attestation=artifact_map["build_attestation"],
            attestation_signature=artifact_map["build_attestation_signature"],
            root=root,
        )
        provenance_path = output_dir / f"{prefix}_provenance.intoto.json"
        artifact_map["provenance"] = write_new_json(provenance_path, provenance, root=root)
        created.append(provenance_path)

        digest_map = _candidate_digest_map(distribution)
        try:
            _validate_updater_zip(updater, artifact_map["installer"], artifact_map["channel_manifest"], artifact_map["channel_manifest_signature"], version, "signed candidate updater")
            attestation_document = _validate_build_attestation(
                attestation_path, target_release=version, candidate_id=identity,
                artifact_digests=digest_map, signing_identity_id=str(signer["identity_id"]),
                sbom_sha256=str(artifact_map["sbom"]["sha256"]), label="signed candidate build attestation",
            )
            _validate_sbom(sbom_path, target_release=version, candidate_id=identity, artifact_digests=digest_map, label="signed candidate SBOM")
            _validate_provenance(
                provenance_path, target_release=version, candidate_id=identity, artifact_digests=digest_map,
                sbom_sha256=str(artifact_map["sbom"]["sha256"]), signing_identity_id=str(signer["identity_id"]),
                build_attestation=artifact_map["build_attestation"], build_attestation_signature=artifact_map["build_attestation_signature"],
                build_attestation_document=attestation_document,
                label="signed candidate provenance",
            )
        except ValueError as error:
            fail(f"Generated candidate trust contract is invalid: {error}")

        signature_evidence: list[dict[str, object]] = []
        for role in ("desktop", "cli", "installer"):
            report_path = output_dir / f"{prefix}_{role}_signtool.json"
            report = {
                "schema_version": 1,
                "target_release": version,
                "candidate_id": identity,
                "role": role,
                "artifact_sha256": artifact_map[role]["sha256"],
                "authenticode_valid": True,
                "verification_tool": "signtool_and_windows_authenticode",
                **copy_verification[role],
                "warnings": [],
            }
            report_descriptor = write_new_json(report_path, report, root=root)
            created.append(report_path)
            signature_evidence.append({"role": role, "artifact_sha256": artifact_map[role]["sha256"], "report": report_descriptor})

        manifest = {
            "schema_version": 1,
            "target_release": version,
            "candidate_id": identity,
            "payload_id": payload_id,
            "signing_identity_id": signer["identity_id"],
            "signing_identity": signer["descriptor"],
            "artifacts": [{"role": role, **artifact_map[role]} for role in sorted(artifact_map)],
            "signature_evidence": signature_evidence,
            "detached_signature_evidence": [
                {"role": "build_attestation", "signature_role": "build_attestation_signature", "verification": attestation_signature_verification},
                {"role": "channel_manifest", "signature_role": "channel_manifest_signature", "verification": channel_signature_verification},
            ],
        }
        manifest_path = output_dir / f"{prefix}_candidate_manifest.json"
        manifest_descriptor = write_new_json(manifest_path, manifest, root=root)
        created.append(manifest_path)
        return {
            "schema_version": 2,
            "passed": True,
            "channel": channel,
            "target_release": version,
            "candidate_id": identity,
            "payload_id": payload_id,
            "candidate_manifest": manifest_descriptor,
            "signing_identity_id": signer["identity_id"],
            "authenticode_verified_on_sources_and_copies": True,
            "signed_channel_manifest_verified": True,
            "protected_build_attestation_verified": True,
            "stable_authorized": False,
            "note": "Candidate assembly does not approve release; commercial readiness and final approval remain mandatory.",
        }
    except BaseException:
        for path in reversed(created):
            path.unlink(missing_ok=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--channel", required=True, choices=("beta", "stable"))
    parser.add_argument("--label", required=True)
    parser.add_argument("--desktop", type=Path, default=ROOT / "target/release/quickpls-desktop.exe")
    parser.add_argument("--cli", type=Path, default=ROOT / "target/release/qpls.exe")
    parser.add_argument("--installer", type=Path, default=ROOT / "target/release/bundle/nsis" / "missing-until-version-resolved.exe")
    parser.add_argument("--build-started-at", required=True)
    parser.add_argument("--build-finished-at", required=True)
    parser.add_argument("--minimum-installed-version", required=True)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "release/candidates")
    parser.add_argument("--signtool", default=None)
    args = parser.parse_args()
    if args.installer.name == "missing-until-version-resolved.exe":
        version, _ = read_version_contract(ROOT)
        args.installer = ROOT / "target/release/bundle/nsis" / f"QuickPLS_{version}_x64-setup.exe"
    print(json.dumps(build_signed_candidate(
        channel=args.channel, label=args.label, desktop=args.desktop, cli=args.cli, installer=args.installer,
        minimum_installed_version=args.minimum_installed_version,
        build_started_at=args.build_started_at, build_finished_at=args.build_finished_at,
        output_dir=args.output_dir, signtool=args.signtool,
    ), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
