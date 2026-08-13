from __future__ import annotations

import copy
import hashlib
import json
import struct
import tempfile
import unittest
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from unittest import mock

from validation import quickpls_3_release_readiness as readiness
from validation import quickpls_signed_candidate as signed_candidate


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_PATH = ROOT / "validation" / "quickpls_3_release_readiness.json"
NOW = datetime(2026, 8, 13, 12, 0, tzinfo=timezone.utc)
LEAF_SUBJECT = "CN=QuickPLS Release Publisher, O=QuickPLS"
LEAF_THUMBPRINT = "A" * 40
SIGNER_ID = readiness._approved_signer_identity_id(LEAF_SUBJECT, LEAF_THUMBPRINT)


def repository_contract() -> dict:
    return json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_sha256(value: object) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def write_artifact(root: Path, relative: str, content: bytes) -> dict:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    return {"path": relative, "size": len(content), "sha256": sha256(path)}


def minimal_signed_pe(marker: bytes) -> bytes:
    data = bytearray(0x210)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 0x80)
    data[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<HHIIIHH", data, 0x84, 0x8664, 1, 0, 0, 0, 0xF0, 0x0022)
    optional = 0x98
    struct.pack_into("<H", data, optional, 0x20B)
    struct.pack_into("<I", data, optional + 108, 16)
    struct.pack_into("<II", data, optional + 144, 0x200, 16)
    data[0x188:0x190] = b".text\0\0\0"
    struct.pack_into("<IHH", data, 0x200, 16, 0x0200, 0x0002)
    data[0x208:0x210] = marker[:8].ljust(8, b"!")
    return bytes(data)


def trusted_signtool_execution(path: Path) -> dict[str, object]:
    resolved = str(path.resolve())
    return {
        "returncode": 0,
        "stdout": (
            f"Verifying: {resolved}\n"
            "Signing Certificate Chain:\n"
            "    Issued to: QuickPLS Release Publisher\n"
            "    Issued by: Trusted Test Code Signing CA\n"
            "The signature is timestamped: 2026-08-13T10:45:00+00:00\n"
            "Timestamp Verified by:\n"
            "    Issued to: Trusted Test Timestamp CA\n"
            f"Successfully verified: {resolved}\n"
        ),
        "stderr": "",
    }


def trusted_windows_identity(path: Path) -> dict[str, object]:
    name = path.name.casefold()
    if "cli" in name or name == "qpls.exe":
        original = "qpls.exe"
    elif "setup" in name or "installer" in name:
        original = "QuickPLS_3.0.0_x64-setup.exe"
    else:
        original = "quickpls-desktop.exe"
    return {
        "signature_status": "Valid",
        "leaf_subject": LEAF_SUBJECT,
        "leaf_sha1_thumbprint": LEAF_THUMBPRINT,
        "product_name": "QuickPLS",
        "product_version": "3.0.0",
        "file_version": "3.0.0.0",
        "original_filename": original,
    }


def trusted_cms_execution(payload: Path, signature: Path) -> dict[str, object]:
    return {
        "exit_code": 0,
        "verification_output": "Trusted detached CMS signature",
        "leaf_subject": LEAF_SUBJECT,
        "leaf_sha1_thumbprint": LEAF_THUMBPRINT,
    }


def write_approved_signer(root: Path) -> dict[str, object]:
    relative = Path("validation/quickpls_signing_identity.json")
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({
        "schema_version": 1,
        "document_type": "quickpls_authenticode_signing_identity",
        "status": "approved",
        "identity_id": SIGNER_ID,
        "leaf_subject": LEAF_SUBJECT,
        "leaf_sha1_thumbprint": LEAF_THUMBPRINT,
        "approved_by": "QuickPLS release board",
        "approved_at": "2026-08-13T09:00:00+00:00",
        "key_protection": "managed_signing_service",
        "notes": "Test-only approved signer fixture.",
    }), encoding="utf-8")
    return {"path": relative.as_posix(), "size": path.stat().st_size, "sha256": sha256(path)}


def trusted_signature_fields(path: Path) -> dict[str, object]:
    execution = trusted_signtool_execution(path)
    output = readiness._normalize_signtool_output(
        str(execution["stdout"]), str(execution["stderr"]), path
    )
    return {
        "command": list(readiness.SIGNTOOL_ARGUMENTS),
        "exit_code": 0,
        "verification_output": output,
        "verification_output_sha256": hashlib.sha256(output.encode()).hexdigest(),
        "timestamp": "2026-08-13T10:45:00+00:00",
        "verified_file_sha256": sha256(path),
        "signer_identity_id": SIGNER_ID,
        "leaf_subject": LEAF_SUBJECT,
        "leaf_sha1_thumbprint": LEAF_THUMBPRINT,
        "product_name": "QuickPLS",
        "product_version": "3.0.0",
        "file_version": "3.0.0.0",
        "original_filename": trusted_windows_identity(path)["original_filename"],
    }


def build_candidate(
    root: Path,
    variant: str = "primary",
    *,
    fake_pe_role: str | None = None,
    invalid_zip: bool = False,
    invalid_sbom: bool = False,
    signature_mismatch: bool = False,
    publisher_mismatch: bool = False,
    timestamp_mismatch: bool = False,
    target_release: str = "3.0.0",
    channel: str = "stable",
) -> tuple[str, dict]:
    signer_descriptor = write_approved_signer(root)
    artifact_map: dict[str, dict] = {}
    for role in sorted(readiness.SIGNED_PE_ROLES):
        payload = (
            f"not-a-pe {role}\n".encode()
            if fake_pe_role == role
            else minimal_signed_pe(f"{variant}-{role}".encode())
        )
        artifact_map[role] = write_artifact(
            root,
            f"validation/results/candidate-{variant}/{role}.exe",
            payload,
        )

    payload_id = readiness._candidate_payload_identity(target_release, artifact_map)
    channel_manifest = {
        "schema_version": 1,
        "document_type": "quickpls_signed_channel_manifest",
        "channel": channel,
        "target_release": target_release,
        "payload_id": payload_id,
        "signing_identity_id": SIGNER_ID,
        "minimum_installed_version": target_release,
        "allow_downgrade": False,
        "manual_check_default": True,
        "installer": artifact_map["installer"],
        "recovery": {"mode": "offline_full_installer", "full_installer_sha256": artifact_map["installer"]["sha256"]},
    }
    artifact_map["channel_manifest"] = write_artifact(
        root, f"validation/results/candidate-{variant}/channel-manifest.json", json.dumps(channel_manifest).encode()
    )
    artifact_map["channel_manifest_signature"] = write_artifact(
        root, f"validation/results/candidate-{variant}/channel-manifest.p7s", f"signed-{variant}".encode()
    )

    updater_relative = f"validation/results/candidate-{variant}/updater_bundle.zip"
    updater_path = root / updater_relative
    updater_path.parent.mkdir(parents=True, exist_ok=True)
    if invalid_zip:
        updater_path.write_bytes(b"not a ZIP archive")
    else:
        installer_path = root / artifact_map["installer"]["path"]
        with zipfile.ZipFile(updater_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr(f"QuickPLS_{target_release}_x64-setup.exe", installer_path.read_bytes())
            archive.writestr("quickpls-channel-manifest.json", (root / artifact_map["channel_manifest"]["path"]).read_bytes())
            archive.writestr("quickpls-channel-manifest.p7s", (root / artifact_map["channel_manifest_signature"]["path"]).read_bytes())
    artifact_map["updater_bundle"] = {"path": updater_relative, "size": updater_path.stat().st_size, "sha256": sha256(updater_path)}
    candidate_id = readiness._candidate_distribution_identity(target_release, artifact_map)
    distribution_digests = readiness._candidate_digest_map(artifact_map)

    sbom_relative = f"validation/results/candidate-{variant}/sbom.json"
    if invalid_sbom:
        artifact_map["sbom"] = write_artifact(root, sbom_relative, b'{"schema_version":')
    else:
        component = {
            "type": "library", "bom-ref": "pkg:npm/react@19.1.1", "name": "react", "version": "19.1.1",
            "purl": "pkg:npm/react@19.1.1", "licenses": [{"expression": "MIT"}],
            "properties": [{"name": "quickpls:ecosystem", "value": "npm"}],
        }
        sbom = signed_candidate.candidate_sbom(
            version=target_release, identity=candidate_id, distribution=artifact_map,
            components=[component], dependency_graph={component["bom-ref"]: []}, generated_at="2026-08-13T10:30:00Z",
        )
        artifact_map["sbom"] = write_artifact(root, sbom_relative, json.dumps(sbom).encode())

    build_attestation = {
        "schema_version": 1,
        "document_type": "quickpls_protected_build_attestation",
        "target_release": target_release,
        "candidate_id": candidate_id,
        "candidate_artifact_digests": distribution_digests,
        "signing_identity_id": SIGNER_ID,
        "sbom_sha256": artifact_map["sbom"]["sha256"],
        "source_commit": "a" * 40,
        "source_tree_clean": True,
        "protected_build": {
            "workflow_id": "owner/quickpls/.github/workflows/release.yml",
            "workflow_run_id": "123456789",
            "workflow_ref": "owner/quickpls/.github/workflows/release.yml@refs/heads/main",
            "repository": "owner/quickpls",
            "runner_environment": "github-hosted",
            "oidc_subject": "repo:owner/quickpls:ref:refs/heads/main",
        },
        "build_id": "github-actions:owner/quickpls:123456789",
        "builder_identity": "github-actions:owner/quickpls/.github/workflows/release.yml@refs/heads/main:github-hosted",
        "build_started_at": "2026-08-13T10:00:00+00:00",
        "build_finished_at": "2026-08-13T10:30:00+00:00",
        "toolchain": {"rustc": "rustc 1", "cargo": "cargo 1", "node": "node 24", "npm": "npm 11", "tauri_cli": "tauri 2"},
        "lockfiles": {"Cargo.lock": "b" * 64, "package-lock.json": "c" * 64},
    }
    artifact_map["build_attestation"] = write_artifact(
        root, f"validation/results/candidate-{variant}/build-attestation.json", json.dumps(build_attestation).encode()
    )
    artifact_map["build_attestation_signature"] = write_artifact(
        root, f"validation/results/candidate-{variant}/build-attestation.p7s", f"attested-{variant}".encode()
    )

    provenance = {
        "_type": "https://in-toto.io/Statement/v1",
        "subject": [{"name": role, "digest": {"sha256": digest}} for role, digest in sorted(distribution_digests.items())],
        "predicateType": "https://slsa.dev/provenance/v1",
        "predicate": {
            "buildDefinition": {
                "buildType": "https://quickpls.org/build-types/windows-protected-release/v1",
                "externalParameters": {"target_release": target_release, "channel": "signed", "candidate_id": candidate_id},
                "internalParameters": {"signing_identity_id": SIGNER_ID},
                "resolvedDependencies": [
                    {"uri": "git+repository", "digest": {"gitCommit": "a" * 40}},
                    {"uri": "file:Cargo.lock", "digest": {"sha256": "b" * 64}},
                    {"uri": "file:package-lock.json", "digest": {"sha256": "c" * 64}},
                ],
            },
            "runDetails": {
                "builder": {"id": "github-actions:owner/quickpls/.github/workflows/release.yml@refs/heads/main"},
                "metadata": {"invocationId": "github-actions:owner/quickpls:123456789", "startedOn": "2026-08-13T10:00:00+00:00", "finishedOn": "2026-08-13T10:30:00+00:00"},
                "byproducts": [
                    {"name": "cyclonedx-sbom", "digest": {"sha256": artifact_map["sbom"]["sha256"]}},
                    {"name": "protected-build-attestation", "digest": {"sha256": artifact_map["build_attestation"]["sha256"]}},
                    {"name": "protected-build-attestation-signature", "digest": {"sha256": artifact_map["build_attestation_signature"]["sha256"]}},
                ],
            },
        },
    }
    artifact_map["provenance"] = write_artifact(
        root,
        f"validation/results/candidate-{variant}/provenance.json",
        json.dumps(provenance).encode(),
    )

    signatures = []
    for role in sorted(readiness.SIGNED_PE_ROLES):
        pe_path = root / artifact_map[role]["path"]
        runtime_fields = trusted_signature_fields(pe_path)
        if publisher_mismatch and role == "desktop":
            runtime_fields["leaf_subject"] = "CN=Different Publisher"
        if timestamp_mismatch and role == "desktop":
            runtime_fields["timestamp"] = "2026-08-12T10:45:00+00:00"
        signed_hash = "0" * 64 if signature_mismatch and role == "desktop" else artifact_map[role]["sha256"]
        report = {
            "schema_version": 1,
            "target_release": target_release,
            "candidate_id": candidate_id,
            "role": role,
            "artifact_sha256": signed_hash,
            "authenticode_valid": True,
            "verification_tool": "signtool_and_windows_authenticode",
            "warnings": [],
            **runtime_fields,
        }
        report_descriptor = write_artifact(
            root,
            f"validation/results/candidate-{variant}/signature-{role}.json",
            json.dumps(report).encode(),
        )
        signatures.append(
            {
                "role": role,
                "artifact_sha256": artifact_map[role]["sha256"],
                "report": report_descriptor,
            }
        )

    artifacts = [{"role": role, **artifact_map[role]} for role in sorted(readiness.REQUIRED_CANDIDATE_ROLES)]
    manifest_relative = f"validation/results/candidate-{variant}-manifest.json"
    manifest_path = root / manifest_relative
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "target_release": target_release,
                "candidate_id": candidate_id,
                "payload_id": payload_id,
                "signing_identity_id": SIGNER_ID,
                "signing_identity": signer_descriptor,
                "artifacts": artifacts,
                "signature_evidence": signatures,
                "detached_signature_evidence": [
                    {
                        "role": "build_attestation", "signature_role": "build_attestation_signature",
                        "verification": {
                            "verification_tool": "windows_signed_cms", "exit_code": 0,
                            "verification_output": "Trusted detached CMS signature",
                            "verification_output_sha256": hashlib.sha256(b"Trusted detached CMS signature").hexdigest(),
                            "signer_identity_id": SIGNER_ID, "leaf_subject": LEAF_SUBJECT,
                            "leaf_sha1_thumbprint": LEAF_THUMBPRINT,
                            "payload_sha256": artifact_map["build_attestation"]["sha256"],
                            "signature_sha256": artifact_map["build_attestation_signature"]["sha256"],
                        },
                    },
                    {
                        "role": "channel_manifest", "signature_role": "channel_manifest_signature",
                        "verification": {
                            "verification_tool": "windows_signed_cms", "exit_code": 0,
                            "verification_output": "Trusted detached CMS signature",
                            "verification_output_sha256": hashlib.sha256(b"Trusted detached CMS signature").hexdigest(),
                            "signer_identity_id": SIGNER_ID, "leaf_subject": LEAF_SUBJECT,
                            "leaf_sha1_thumbprint": LEAF_THUMBPRINT,
                            "payload_sha256": artifact_map["channel_manifest"]["sha256"],
                            "signature_sha256": artifact_map["channel_manifest_signature"]["sha256"],
                        },
                    },
                ],
            }
        ),
        encoding="utf-8",
    )
    descriptor = {
        "path": manifest_relative,
        "size": manifest_path.stat().st_size,
        "sha256": sha256(manifest_path),
    }
    return candidate_id, descriptor


def evidence_record(
    contract: dict,
    root: Path,
    requirement_id: str,
    *,
    candidate: tuple[str, dict] | None = None,
    performed_at: str = "2026-08-13T11:00:00+00:00",
) -> dict:
    requirement = next(item for item in contract["requirements"] if item["id"] == requirement_id)
    identity_key = readiness.EXPECTED_IDENTITY[requirement_id]
    if requirement_id in readiness.CANDIDATE_SCOPED_IDS and candidate is None:
        candidate = build_candidate(root)
    candidate_id = candidate[0] if candidate is not None and requirement_id in readiness.CANDIDATE_SCOPED_IDS else None
    criterion_results = [
        {
            "check_id": check_id,
            "passed": True,
            "result": {
                "summary": f"Verified {check_id}.",
                "measurements": {"executed": True, "failures": 0},
            },
        }
        for check_id in requirement["acceptance_check_ids"]
    ]
    if identity_key == "artifact_identity":
        gate_report = {
            "schema_version": 1,
            "report_type": "quickpls_release_gate",
            "requirement_id": requirement_id,
            "target_release": "3.0.0",
            "candidate_id": candidate_id,
            "criterion_results": criterion_results,
        }
        artifact = write_artifact(
            root,
            f"validation/results/gate-artifacts/{requirement_id.replace('.', '-')}.json",
            json.dumps(gate_report).encode(),
        )
        identity = {
            "name": f"QuickPLS structured gate report for {requirement_id}",
            "identifier": artifact["sha256"],
            "artifact": artifact,
        }
    else:
        identity = {
            "name": f"Qualified reviewer for {requirement_id}",
            "role": "independent release reviewer",
            "organization": "Independent Method and Product Review Group",
            "independence": "independent",
            "conflict_disclosure": "No relevant financial, authorship, or product conflict disclosed.",
            "disposition": "approved",
            "reviewed_scope": f"QuickPLS 3.0.0 evidence and acceptance checks for {requirement_id}.",
            "record_id": f"QPLS3/{requirement_id}/20260813",
        }
    record = {
        "schema_version": 1,
        "requirement_id": requirement_id,
        "target_release": "3.0.0",
        "passed": True,
        "performed_at": performed_at,
        "scope": f"Release acceptance for {requirement_id}",
        "summary": "Every declared acceptance check executed successfully against the named candidate or review scope.",
        "criterion_results": criterion_results,
        identity_key: identity,
    }
    if requirement_id in readiness.CANDIDATE_SCOPED_IDS:
        record["candidate_id"] = candidate[0]
        record["candidate_manifest"] = candidate[1]
    return record


def attach_record(
    contract: dict,
    root: Path,
    requirement_id: str,
    *,
    candidate: tuple[str, dict] | None = None,
    record: dict | None = None,
    raw: str | None = None,
    filename: str | None = None,
    performed_at: str = "2026-08-13T11:00:00+00:00",
) -> Path:
    item = next(row for row in contract["requirements"] if row["id"] == requirement_id)
    relative = Path("validation") / "results" / (filename or f"{requirement_id.replace('.', '_')}.json")
    destination = root / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    if raw is None:
        destination.write_text(
            json.dumps(
                record
                or evidence_record(
                    contract,
                    root,
                    requirement_id,
                    candidate=candidate,
                    performed_at=performed_at,
                )
            ),
            encoding="utf-8",
        )
    else:
        destination.write_text(raw, encoding="utf-8")
    item["status"] = "passed"
    item["evidence"] = [{"type": "file", "ref": relative.as_posix()}]
    return destination


def competitor_inputs(contract: dict, root: Path, candidate_id: str) -> dict:
    evidence_records = []
    for requirement in contract["requirements"]:
        for evidence in requirement["evidence"]:
            if evidence["type"] == "file":
                path = root / evidence["ref"]
                evidence_records.append(
                    {
                        "requirement_id": requirement["id"],
                        "path": evidence["ref"],
                        "sha256": sha256(path),
                    }
                )
    evidence_records.sort(key=lambda item: (item["requirement_id"], item["path"]))
    contract_inputs = {
        "schema_version": contract["schema_version"],
        "target_release": contract["target_release"],
        "target_channel": contract["target_channel"],
        "product_policy": contract["product_policy"],
        "release_channels": contract["release_channels"],
        "signing_identity_policy": contract["signing_identity_policy"],
        "evidence_policy": contract["evidence_policy"],
        "requirements": [
            {
                "id": item["id"],
                "status": item["status"],
                "acceptance_check_ids": item["acceptance_check_ids"],
                "evidence": item["evidence"],
            }
            for item in contract["requirements"]
        ],
    }
    return {
        "target_channel": "stable",
        "required_requirement_ids": sorted(item["id"] for item in contract["requirements"]),
        "evidence_records": evidence_records,
        "candidate_id": candidate_id,
        "contract_inputs_sha256": canonical_sha256(contract_inputs),
    }


def build_ready_contract(
    root: Path,
    *,
    evidence_at: str = "2026-08-13T11:00:00+00:00",
    approved_at: str = "2026-08-13T12:00:00+00:00",
) -> tuple[dict, tuple[str, dict]]:
    contract = repository_contract()
    contract["overall_status"] = "passed"
    candidate = build_candidate(root)
    for requirement in contract["requirements"]:
        attach_record(
            contract,
            root,
            requirement["id"],
            candidate=candidate,
            performed_at=evidence_at,
        )
    decision_relative = Path("validation/results/release_decision.json")
    decision = root / decision_relative
    decision.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "target_release": "3.0.0",
                "approved": True,
                "approved_by": "release manager",
                "approved_at": approved_at,
                "summary": "All commercial-readiness inputs are bound and approved after final evidence.",
                "competitor_ready_inputs": competitor_inputs(contract, root, candidate[0]),
            }
        ),
        encoding="utf-8",
    )
    contract["release_decision"] = {
        "status": "approved",
        "approved_by": "release manager",
        "approved_at": approved_at,
        "record": decision_relative.as_posix(),
    }
    return contract, candidate


class RepositoryContractTests(unittest.TestCase):
    def test_contract_is_valid_and_deliberately_not_release_ready(self) -> None:
        report = readiness.load_and_validate(CONTRACT_PATH, repository_root=ROOT)

        self.assertTrue(report["structurally_valid"])
        self.assertFalse(report["competitor_ready"])
        self.assertFalse(report["release_ready"])
        self.assertEqual(report["counts"], {"passed": 0, "pending": 18, "failed": 0})
        self.assertEqual(set(report["pending"]), readiness.REQUIRED_IDS)
        self.assertEqual(report["release_decision"], "pending")
        self.assertEqual(
            report["offline_claims"],
            {
                "functional_offline": True,
                "quickpls_app_page_external_requests": False,
                "strict_process_tree_zero_egress": False,
                "strict_gate_status": "pending",
            },
        )

    def test_strict_offline_claims_cannot_bypass_the_os_containment_gate(self) -> None:
        for mutation in (
            lambda policy: policy["strict_zero_egress_claim_gate"].__setitem__("status", "passed"),
            lambda policy: policy["strict_zero_egress_claim_gate"].__setitem__(
                "application_level_containment_sufficient", True
            ),
            lambda policy: policy["prohibited_claims"].remove(
                "no_telemetry_without_os_enforced_fixed_webview2_containment"
            ),
            lambda policy: policy.__setitem__(
                "application_network_behavior", "all_processes_make_no_external_requests"
            ),
        ):
            contract = repository_contract()
            mutation(contract["product_policy"])
            with self.assertRaises(readiness.ContractError):
                readiness.validate_contract(contract, repository_root=ROOT, now=NOW)


class SignedFactoryReadinessIntegrationTests(unittest.TestCase):
    def test_durable_factory_manifest_is_consumed_by_readiness(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "Cargo.lock").write_text("test cargo lock", encoding="utf-8")
            (root / "package-lock.json").write_text("{}", encoding="utf-8")
            signer_descriptor = write_approved_signer(root)
            signer = {
                "status": "approved",
                "identity_id": SIGNER_ID,
                "leaf_subject": LEAF_SUBJECT,
                "leaf_sha1_thumbprint": LEAF_THUMBPRINT,
                "descriptor": signer_descriptor,
            }
            inputs = root / "inputs"
            inputs.mkdir()
            desktop = inputs / "quickpls-desktop.exe"
            cli = inputs / "qpls.exe"
            installer = inputs / "QuickPLS_3.0.0_x64-setup.exe"
            desktop.write_bytes(minimal_signed_pe(b"desktop"))
            cli.write_bytes(minimal_signed_pe(b"cli"))
            installer.write_bytes(minimal_signed_pe(b"setup"))

            def fake_signature(_tool, path, *, role, version, signer):
                self.assertEqual(version, "3.0.0")
                self.assertEqual(signer["identity_id"], SIGNER_ID)
                return trusted_signature_fields(path)

            def fake_cms(payload, signature, *, signer):
                signature.write_bytes(b"CMS:" + hashlib.sha256(payload.read_bytes()).digest())
                return {
                    "verification_tool": "windows_signed_cms",
                    "exit_code": 0,
                    "verification_output": "Trusted detached CMS signature",
                    "verification_output_sha256": hashlib.sha256(b"Trusted detached CMS signature").hexdigest(),
                    "signer_identity_id": SIGNER_ID,
                    "leaf_subject": LEAF_SUBJECT,
                    "leaf_sha1_thumbprint": LEAF_THUMBPRINT,
                    "payload_sha256": sha256(payload),
                    "signature_sha256": sha256(signature),
                }

            component = {
                "type": "library", "bom-ref": "pkg:npm/react@19.1.1", "name": "react", "version": "19.1.1",
                "purl": "pkg:npm/react@19.1.1", "licenses": [{"expression": "MIT"}],
                "properties": [{"name": "quickpls:ecosystem", "value": "npm"}],
            }
            protected_context = {
                "workflow_id": "owner/quickpls/.github/workflows/release.yml",
                "workflow_run_id": "123456789",
                "workflow_ref": "owner/quickpls/.github/workflows/release.yml@refs/heads/main",
                "repository": "owner/quickpls",
                "runner_environment": "github-hosted",
                "oidc_subject": "repo:owner/quickpls:ref:refs/heads/main",
            }
            channel_policy = {"beta": {"artifact_factory": "signed_candidate"}, "stable": {"artifact_factory": "signed_candidate"}}
            with (
                mock.patch.object(signed_candidate, "read_version_contract", return_value=("3.0.0", {})),
                mock.patch.object(signed_candidate, "read_release_channel_contract", return_value={"channels": channel_policy}),
                mock.patch.object(signed_candidate, "git_identity", return_value={"clean": True, "commit": "a" * 40}),
                mock.patch.object(signed_candidate, "protected_build_context", return_value=protected_context),
                mock.patch.object(signed_candidate, "approved_signer", return_value=signer),
                mock.patch.object(signed_candidate, "locate_signtool", return_value="C:/test/signtool.exe"),
                mock.patch.object(signed_candidate, "verify_signature", side_effect=fake_signature),
                mock.patch.object(signed_candidate, "sign_detached_cms", side_effect=fake_cms),
                mock.patch.object(signed_candidate, "npm_components", return_value=([component], {component["bom-ref"]: []})),
                mock.patch.object(signed_candidate, "run_cargo_metadata", return_value={}),
                mock.patch.object(signed_candidate, "cargo_components", return_value=([], {})),
                mock.patch.object(signed_candidate, "tool_version", return_value="tool 1.0"),
            ):
                factory = signed_candidate.build_signed_candidate(
                    channel="stable", label="e2e", desktop=desktop, cli=cli, installer=installer,
                    minimum_installed_version="2.46.0",
                    build_started_at="2026-08-13T10:00:00Z", build_finished_at="2026-08-13T10:30:00Z",
                    output_dir=root / "release/candidates", root=root,
                )

            self.assertTrue(str(factory["candidate_manifest"]["path"]).startswith("release/candidates/"))
            contract = repository_contract()
            candidate = (factory["candidate_id"], factory["candidate_manifest"])
            attach_record(contract, root, "signing.artifacts", candidate=candidate)
            with (
                mock.patch.object(readiness, "_run_signtool", side_effect=trusted_signtool_execution),
                mock.patch.object(readiness, "_run_windows_file_identity", side_effect=trusted_windows_identity),
                mock.patch.object(readiness, "_run_windows_cms_verification", side_effect=trusted_cms_execution),
            ):
                report = readiness.validate_contract(contract, repository_root=root, now=NOW)
            self.assertEqual(report["candidate_id"], factory["candidate_id"])
            self.assertFalse(report["release_ready"])


class ProductionSignToolSeamTests(unittest.TestCase):
    def test_missing_signtool_fails_closed(self) -> None:
        with mock.patch.object(readiness, "_locate_signtool", return_value=None):
            with self.assertRaisesRegex(readiness.ContractError, "SignTool was not found"):
                readiness._run_signtool(Path("candidate.exe"))

    def test_signtool_invocation_uses_frozen_trust_and_timestamp_policy(self) -> None:
        candidate = Path("candidate.exe").resolve()
        signtool = Path("C:/Windows-Kits/signtool.exe").resolve()
        completed = mock.Mock(returncode=0, stdout="verified", stderr="")
        with (
            mock.patch.object(readiness, "_locate_signtool", return_value=signtool),
            mock.patch.object(readiness.subprocess, "run", return_value=completed) as run,
        ):
            result = readiness._run_signtool(candidate)

        self.assertEqual(result, {"returncode": 0, "stdout": "verified", "stderr": ""})
        run.assert_called_once_with(
            [str(signtool), *readiness.SIGNTOOL_ARGUMENTS, str(candidate)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=120,
            check=False,
            shell=False,
        )

    def test_signtool_parser_rejects_multiple_signing_chains(self) -> None:
        output = str(trusted_signtool_execution(Path("candidate.exe"))["stdout"])
        duplicate = output.replace(
            "Signing Certificate Chain:\n",
            "Signing Certificate Chain:\n    Issued to: Other Publisher\nThe signature is timestamped: older\nSigning Certificate Chain:\n",
            1,
        )
        with self.assertRaisesRegex(readiness.ContractError, "exactly one signing certificate chain"):
            readiness._parse_signtool_identity(duplicate, "candidate")


class ProductionWindowsTrustSeamTests(unittest.TestCase):
    def test_leaf_and_pe_identity_uses_literal_env_path_and_utf8_json(self) -> None:
        candidate = Path("C:/candidate's dir/quickpls-desktop.exe")
        powershell = Path("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
        completed = mock.Mock(returncode=0, stdout=json.dumps(trusted_windows_identity(candidate)), stderr="")
        with (
            mock.patch.object(readiness, "_locate_powershell", return_value=powershell),
            mock.patch.object(readiness.subprocess, "run", return_value=completed) as run,
        ):
            result = readiness._run_windows_file_identity(candidate)
        self.assertEqual(result["leaf_sha1_thumbprint"], LEAF_THUMBPRINT)
        arguments = run.call_args.args[0]
        self.assertNotIn(str(candidate.resolve()), arguments)
        self.assertEqual(run.call_args.kwargs["env"]["QPLS_VERIFY_FILE"], str(candidate.resolve()))
        self.assertFalse(run.call_args.kwargs["shell"])

    def test_detached_cms_verifier_uses_two_literal_env_paths(self) -> None:
        payload = Path("C:/candidate's dir/manifest.json")
        signature = Path("C:/candidate's dir/manifest.p7s")
        powershell = Path("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
        completed = mock.Mock(
            returncode=0,
            stdout=json.dumps({"leaf_subject": LEAF_SUBJECT, "leaf_sha1_thumbprint": LEAF_THUMBPRINT}),
            stderr="",
        )
        with (
            mock.patch.object(readiness, "_locate_powershell", return_value=powershell),
            mock.patch.object(readiness.subprocess, "run", return_value=completed) as run,
        ):
            result = readiness._run_windows_cms_verification(payload, signature)
        self.assertEqual(result["exit_code"], 0)
        environment = run.call_args.kwargs["env"]
        self.assertEqual(environment["QPLS_CMS_PAYLOAD"], str(payload.resolve()))
        self.assertEqual(environment["QPLS_CMS_SIGNATURE"], str(signature.resolve()))


class FailClosedValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.signtool_patcher = mock.patch.object(
            readiness,
            "_run_signtool",
            side_effect=trusted_signtool_execution,
        )
        self.signtool_mock = self.signtool_patcher.start()
        self.addCleanup(self.signtool_patcher.stop)
        self.windows_identity_patcher = mock.patch.object(
            readiness, "_run_windows_file_identity", side_effect=trusted_windows_identity
        )
        self.windows_identity_mock = self.windows_identity_patcher.start()
        self.addCleanup(self.windows_identity_patcher.stop)
        self.cms_patcher = mock.patch.object(
            readiness, "_run_windows_cms_verification", side_effect=trusted_cms_execution
        )
        self.cms_mock = self.cms_patcher.start()
        self.addCleanup(self.cms_patcher.stop)

    def test_independent_scientific_review_is_mandatory_and_cannot_be_omitted(self) -> None:
        contract = repository_contract()
        contract["requirements"] = [
            item for item in contract["requirements"] if item["id"] != "science.independent_review"
        ]
        with self.assertRaises(readiness.ContractError):
            readiness.validate_contract(contract, repository_root=ROOT, now=NOW)

    def test_rejects_missing_duplicate_and_unknown_requirements(self) -> None:
        mutations = []
        missing = repository_contract()
        missing["requirements"].pop()
        mutations.append(missing)
        duplicate = repository_contract()
        duplicate["requirements"][-1] = copy.deepcopy(duplicate["requirements"][0])
        mutations.append(duplicate)
        unknown = repository_contract()
        unknown["requirements"][-1]["id"] = "governance.unknown"
        mutations.append(unknown)
        for contract in mutations:
            with self.assertRaises(readiness.ContractError):
                readiness.validate_contract(contract, repository_root=ROOT, now=NOW)

    def test_rejects_invalid_status_and_premature_overall_pass(self) -> None:
        invalid = repository_contract()
        invalid["requirements"][0]["status"] = "waived"
        with self.assertRaises(readiness.ContractError):
            readiness.validate_contract(invalid, repository_root=ROOT, now=NOW)
        premature = repository_contract()
        premature["overall_status"] = "passed"
        with self.assertRaises(readiness.ContractError):
            readiness.validate_contract(premature, repository_root=ROOT, now=NOW)

    def test_rejects_pass_without_bound_repository_json(self) -> None:
        no_evidence = repository_contract()
        no_evidence["requirements"][0]["status"] = "passed"
        with self.assertRaises(readiness.ContractError):
            readiness.validate_contract(no_evidence, repository_root=ROOT, now=NOW)
        attestation_only = repository_contract()
        attestation_only["requirements"][0]["status"] = "passed"
        attestation_only["requirements"][0]["evidence"] = [
            {"type": "attestation", "ref": "LEGAL-REVIEW-2026-001"}
        ]
        with self.assertRaises(readiness.ContractError):
            readiness.validate_contract(attestation_only, repository_root=ROOT, now=NOW)

    def test_rejects_missing_unsafe_or_non_json_evidence_file(self) -> None:
        refs = [
            "validation/results/not-created.json",
            "../outside.json",
            "target/release/report.json",
            "validation/results/report.txt",
        ]
        for ref in refs:
            contract = repository_contract()
            contract["requirements"][0]["status"] = "passed"
            contract["requirements"][0]["evidence"] = [{"type": "file", "ref": ref}]
            with self.subTest(ref=ref), self.assertRaises(readiness.ContractError):
                readiness.validate_contract(contract, repository_root=ROOT, now=NOW)

    def test_rejects_wrong_requirement_release_or_passed_value(self) -> None:
        mutations = [
            {"requirement_id": "signing.artifacts"},
            {"target_release": "2.46.0"},
            {"passed": False},
        ]
        for update in mutations:
            with self.subTest(update=update), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                contract = repository_contract()
                record = evidence_record(contract, root, "signing.identity")
                record.update(update)
                attach_record(contract, root, "signing.identity", record=record)
                with self.assertRaises(readiness.ContractError):
                    readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_stale_malformed_duplicate_key_and_nonfinite_records(self) -> None:
        for name in ["stale", "malformed", "duplicate", "nonfinite"]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                contract = repository_contract()
                valid = evidence_record(contract, root, "signing.identity")
                if name == "stale":
                    valid["performed_at"] = "2024-01-01T00:00:00Z"
                    raw = json.dumps(valid)
                elif name == "malformed":
                    raw = '{"schema_version": 1'
                elif name == "duplicate":
                    raw = json.dumps(valid).replace('{"schema_version": 1', '{"schema_version": 1, "schema_version": 1', 1)
                else:
                    raw = json.dumps(valid).replace('"schema_version": 1', '"schema_version": NaN', 1)
                attach_record(contract, root, "signing.identity", raw=raw)
                with self.assertRaises(readiness.ContractError):
                    readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_wrong_artifact_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract = repository_contract()
            record = evidence_record(contract, root, "signing.identity")
            wrong = "0" * 64
            record["artifact_identity"]["identifier"] = wrong
            record["artifact_identity"]["artifact"]["sha256"] = wrong
            attach_record(contract, root, "signing.identity", record=record)
            with self.assertRaises(readiness.ContractError):
                readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_plain_text_criterion_report(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract = repository_contract()
            record = evidence_record(contract, root, "signing.identity")
            descriptor = record["artifact_identity"]["artifact"]
            report_path = root / descriptor["path"]
            report_path.write_text("all checks passed", encoding="utf-8")
            descriptor["size"] = report_path.stat().st_size
            descriptor["sha256"] = sha256(report_path)
            record["artifact_identity"]["identifier"] = descriptor["sha256"]
            attach_record(contract, root, "signing.identity", record=record)
            with self.assertRaises(readiness.ContractError):
                readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_wrong_reviewer_identity_kind(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract = repository_contract()
            record = evidence_record(contract, root, "trust.legal")
            record["artifact_identity"] = {
                "name": "legal memo",
                "identifier": "0" * 64,
                "artifact": {"path": "missing", "size": 1, "sha256": "0" * 64},
            }
            del record["reviewer_identity"]
            attach_record(contract, root, "trust.legal", record=record)
            with self.assertRaises(readiness.ContractError):
                readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_empty_or_unbound_criterion_results(self) -> None:
        for mutation in ["empty_rows", "empty_result", "wrong_check"]:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                contract = repository_contract()
                record = evidence_record(contract, root, "signing.identity")
                if mutation == "empty_rows":
                    record["criterion_results"] = []
                elif mutation == "empty_result":
                    record["criterion_results"][0]["result"] = ""
                else:
                    record["criterion_results"][0]["check_id"] = "signing.identity.unbound"
                attach_record(contract, root, "signing.identity", record=record)
                with self.assertRaises(readiness.ContractError):
                    readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_one_record_reused_across_requirements(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract = repository_contract()
            attach_record(contract, root, "signing.identity", filename="shared.json")
            second = next(item for item in contract["requirements"] if item["id"] == "signing.artifacts")
            second["status"] = "passed"
            second["evidence"] = [{"type": "file", "ref": "validation/results/shared.json"}]
            with self.assertRaisesRegex(readiness.ContractError, "cannot be reused"):
                readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_cross_candidate_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract = repository_contract()
            attach_record(contract, root, "signing.artifacts", candidate=build_candidate(root, "first"))
            attach_record(contract, root, "installer.clean_offline", candidate=build_candidate(root, "second"))
            with self.assertRaisesRegex(readiness.ContractError, "different release candidate"):
                readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_semantically_invalid_candidate_artifacts(self) -> None:
        cases = [
            ("fake-pe", {"fake_pe_role": "desktop"}),
            ("invalid-zip", {"invalid_zip": True}),
            ("invalid-sbom-json", {"invalid_sbom": True}),
            ("signature-mismatch", {"signature_mismatch": True}),
        ]
        for variant, options in cases:
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                contract = repository_contract()
                candidate = build_candidate(root, variant, **options)
                attach_record(contract, root, "signing.artifacts", candidate=candidate)
                with self.assertRaises(readiness.ContractError):
                    readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_signed_attestation_prevents_sbom_and_manifest_rewrite(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract = repository_contract()
            candidate_id, manifest_descriptor = build_candidate(root, "sbom-rewrite")
            manifest_path = root / manifest_descriptor["path"]
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            sbom_row = next(row for row in manifest["artifacts"] if row["role"] == "sbom")
            sbom_path = root / sbom_row["path"]
            sbom_path.write_text(sbom_path.read_text(encoding="utf-8") + "\n", encoding="utf-8")
            sbom_row["size"] = sbom_path.stat().st_size
            sbom_row["sha256"] = sha256(sbom_path)
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            manifest_descriptor["size"] = manifest_path.stat().st_size
            manifest_descriptor["sha256"] = sha256(manifest_path)
            attach_record(contract, root, "signing.artifacts", candidate=(candidate_id, manifest_descriptor))
            with self.assertRaisesRegex(readiness.ContractError, "sbom_sha256"):
                readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_signtool_nonzero_and_untrusted_results(self) -> None:
        executions = {
            "nonzero": {
                "returncode": 1,
                "stdout": "",
                "stderr": "SignTool Error: A certificate chain could not be built.",
            },
            "untrusted": {
                "returncode": 0,
                "stdout": (
                    "Signing Certificate Chain:\n"
                    "    Issued to: QuickPLS Release Publisher\n"
                    "The signature is timestamped: 2026-08-13T10:45:00+00:00\n"
                    "Timestamp Verified by:\n"
                    "    Issued to: Trusted Test Timestamp CA\n"
                    "Successfully verified: candidate.exe\n"
                    "SignTool Error: The signing certificate is not trusted.\n"
                ),
                "stderr": "",
            },
        }
        for variant, execution in executions.items():
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                contract = repository_contract()
                attach_record(
                    contract,
                    root,
                    "signing.artifacts",
                    candidate=build_candidate(root, f"signtool-{variant}"),
                )
                with mock.patch.object(readiness, "_run_signtool", return_value=execution):
                    with self.assertRaises(readiness.ContractError):
                        readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_validation_time_publisher_and_timestamp_mismatches(self) -> None:
        cases = [
            ("publisher", {"publisher_mismatch": True}, "leaf_subject does not match"),
            ("timestamp", {"timestamp_mismatch": True}, "timestamp does not match"),
        ]
        for variant, options, error in cases:
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                contract = repository_contract()
                attach_record(
                    contract,
                    root,
                    "signing.artifacts",
                    candidate=build_candidate(root, variant, **options),
                )
                with self.assertRaisesRegex(readiness.ContractError, error):
                    readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_candidate_hash_change_during_signtool_verification(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract = repository_contract()
            attach_record(
                contract,
                root,
                "signing.artifacts",
                candidate=build_candidate(root, "hash-change"),
            )

            def mutate_candidate(path: Path) -> dict[str, object]:
                path.write_bytes(path.read_bytes() + b"tampered-after-prehash")
                return trusted_signtool_execution(path)

            with mock.patch.object(readiness, "_run_signtool", side_effect=mutate_candidate):
                with self.assertRaisesRegex(readiness.ContractError, "file hash changed during"):
                    readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_failed_requirement_is_structurally_valid_but_blocks_release(self) -> None:
        contract = repository_contract()
        contract["requirements"][0]["status"] = "failed"
        report = readiness.validate_contract(contract, repository_root=ROOT, now=NOW)
        self.assertFalse(report["release_ready"])
        self.assertEqual(report["failed"], ["signing.identity"])

    def test_only_real_rehashed_bound_records_and_post_evidence_approval_can_be_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract, _candidate = build_ready_contract(root)
            report = readiness.validate_contract(contract, repository_root=root, now=NOW)
            self.assertTrue(report["release_ready"])
            self.assertEqual(report["counts"], {"passed": 18, "pending": 0, "failed": 0})
            self.assertEqual(self.signtool_mock.call_count, 3)
            self.assertEqual(
                {call.args[0].name for call in self.signtool_mock.call_args_list},
                {"desktop.exe", "cli.exe", "installer.exe"},
            )

    def test_rejects_approval_before_latest_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract, _candidate = build_ready_contract(
                root,
                evidence_at="2026-08-13T11:00:00+00:00",
                approved_at="2026-08-13T10:00:00+00:00",
            )
            with self.assertRaisesRegex(readiness.ContractError, "must not precede"):
                readiness.validate_contract(contract, repository_root=root, now=NOW)

    def test_rejects_decision_that_does_not_bind_competitor_ready_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            contract, _candidate = build_ready_contract(root)
            decision_path = root / contract["release_decision"]["record"]
            decision = json.loads(decision_path.read_text(encoding="utf-8"))
            decision["competitor_ready_inputs"]["candidate_id"] = "0" * 64
            decision_path.write_text(json.dumps(decision), encoding="utf-8")
            with self.assertRaisesRegex(readiness.ContractError, "does not bind"):
                readiness.validate_contract(contract, repository_root=root, now=NOW)


if __name__ == "__main__":
    unittest.main()
