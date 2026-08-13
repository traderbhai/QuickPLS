from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from validation import quickpls_signed_candidate as signed
from validation import quickpls_3_release_readiness as readiness


class SignedCandidateFoundationTests(unittest.TestCase):
    def distribution(self) -> dict[str, dict[str, object]]:
        return {
            role: {"path": f"release/candidates/{role}.bin", "size": index + 100, "sha256": chr(97 + index) * 64}
            for index, role in enumerate(
                ("desktop", "cli", "installer", "updater_bundle", "channel_manifest", "channel_manifest_signature")
            )
        }

    def test_candidate_and_payload_identities_are_distinct_and_bound(self) -> None:
        artifacts = self.distribution()
        payload = signed._candidate_payload_identity("3.0.0", artifacts)
        candidate = signed._candidate_distribution_identity("3.0.0", artifacts)
        self.assertRegex(payload, r"^[0-9a-f]{64}$")
        self.assertRegex(candidate, r"^[0-9a-f]{64}$")
        self.assertNotEqual(payload, candidate)
        artifacts["channel_manifest_signature"]["size"] = 999
        self.assertEqual(payload, signed._candidate_payload_identity("3.0.0", artifacts))
        self.assertNotEqual(candidate, signed._candidate_distribution_identity("3.0.0", artifacts))

    def test_signtool_and_approved_identity_are_mandatory(self) -> None:
        with self.assertRaises(SystemExit):
            signed.locate_signtool("Z:/definitely/missing/signtool.exe")
        with self.assertRaises(SystemExit):
            signed.approved_signer()

    def test_descriptor_accepts_durable_candidate_and_rejects_target(self) -> None:
        durable = signed.ROOT / "release" / "candidate-descriptor-test.bin"
        target = signed.ROOT / "target" / "candidate-descriptor-test.bin"
        try:
            durable.parent.mkdir(parents=True, exist_ok=True)
            target.parent.mkdir(parents=True, exist_ok=True)
            durable.write_bytes(b"durable")
            target.write_bytes(b"ephemeral")
            self.assertEqual(signed.descriptor(durable)["path"], "release/candidate-descriptor-test.bin")
            with self.assertRaises(SystemExit):
                signed.descriptor(target)
        finally:
            durable.unlink(missing_ok=True)
            target.unlink(missing_ok=True)

    def test_protected_build_context_is_not_caller_metadata(self) -> None:
        with self.assertRaises(SystemExit):
            signed.protected_build_context("a" * 40, {})
        environment = {
            "GITHUB_ACTIONS": "true",
            "GITHUB_REF_PROTECTED": "true",
            "GITHUB_REF": "refs/heads/main",
            "GITHUB_WORKFLOW_REF": "owner/quickpls/.github/workflows/release.yml@refs/heads/main",
            "GITHUB_RUN_ID": "123456789",
            "GITHUB_REPOSITORY": "owner/quickpls",
            "RUNNER_ENVIRONMENT": "github-hosted",
            "GITHUB_SHA": "a" * 40,
        }
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            workflow = root / ".github/workflows/release.yml"
            workflow.parent.mkdir(parents=True)
            workflow.write_text("name: release", encoding="utf-8")
            context = signed.protected_build_context("a" * 40, environment, root=root)
            self.assertEqual(context["workflow_run_id"], "123456789")
            environment["GITHUB_REF_PROTECTED"] = "false"
            with self.assertRaises(SystemExit):
                signed.protected_build_context("a" * 40, environment, root=root)

    def test_actual_cyclonedx_16_contains_purls_and_complete_graph(self) -> None:
        distribution = self.distribution()
        identity = signed._candidate_distribution_identity("3.0.0", distribution)
        component = {
            "type": "library",
            "bom-ref": "pkg:npm/react@19.1.1",
            "name": "react",
            "version": "19.1.1",
            "purl": "pkg:npm/react@19.1.1",
            "licenses": [{"expression": "MIT"}],
            "properties": [{"name": "quickpls:ecosystem", "value": "npm"}],
        }
        sbom = signed.candidate_sbom(
            version="3.0.0",
            identity=identity,
            distribution=distribution,
            components=[component],
            dependency_graph={component["bom-ref"]: []},
            generated_at="2026-08-13T10:30:00Z",
        )
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "sbom.cdx.json"
            path.write_text(json.dumps(sbom), encoding="utf-8")
            signed._validate_sbom(
                path,
                target_release="3.0.0",
                candidate_id=identity,
                artifact_digests=signed._candidate_digest_map(distribution),
                label="test candidate SBOM",
            )
        self.assertEqual(sbom["bomFormat"], "CycloneDX")
        self.assertEqual(sbom["specVersion"], "1.6")
        self.assertEqual(sbom["components"][0]["purl"], "pkg:npm/react@19.1.1")

    def test_candidate_provenance_is_intoto_slsa_and_attestation_bound(self) -> None:
        distribution = self.distribution()
        identity = signed._candidate_distribution_identity("3.0.0", distribution)
        signer = {"identity_id": "f" * 64}
        context = {
            "workflow_id": "owner/quickpls/.github/workflows/release.yml",
            "workflow_run_id": "123",
            "workflow_ref": "owner/quickpls/.github/workflows/release.yml@refs/heads/main",
            "repository": "owner/quickpls",
            "runner_environment": "github-hosted",
            "oidc_subject": "repo:owner/quickpls:ref:refs/heads/main",
        }
        sbom = {"placeholder": True}
        attestation = {"path": "release/candidates/build.json", "size": 10, "sha256": "1" * 64}
        signature = {"path": "release/candidates/build.p7s", "size": 20, "sha256": "2" * 64}
        provenance = signed.provenance_document(
            version="3.0.0", identity=identity, distribution=distribution, signer=signer,
            source_commit="a" * 40, context=context,
            build_started_at="2026-08-13T10:00:00Z", build_finished_at="2026-08-13T10:30:00Z",
            sbom=sbom, attestation=attestation, attestation_signature=signature,
        )
        self.assertEqual(provenance["_type"], "https://in-toto.io/Statement/v1")
        self.assertEqual(provenance["predicateType"], "https://slsa.dev/provenance/v1")
        byproducts = {row["name"]: row["digest"]["sha256"] for row in provenance["predicate"]["runDetails"]["byproducts"]}
        self.assertEqual(byproducts["protected-build-attestation"], attestation["sha256"])
        self.assertEqual(byproducts["protected-build-attestation-signature"], signature["sha256"])

    def test_channel_manifest_rejects_impossible_upgrade_floor(self) -> None:
        installer = {"path": "release/candidates/setup.exe", "size": 100, "sha256": "a" * 64}
        value = {
            "schema_version": 1,
            "document_type": "quickpls_signed_channel_manifest",
            "channel": "stable",
            "target_release": "3.0.0",
            "payload_id": "b" * 64,
            "signing_identity_id": "c" * 64,
            "minimum_installed_version": "4.0.0",
            "allow_downgrade": False,
            "manual_check_default": True,
            "installer": installer,
            "recovery": {"mode": "offline_full_installer", "full_installer_sha256": installer["sha256"]},
        }
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "channel.json"
            path.write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "exceeds the target release"):
                readiness._validate_channel_manifest(
                    path,
                    target_release="3.0.0",
                    payload_id="b" * 64,
                    artifact_map={"installer": installer},
                    signing_identity_id="c" * 64,
                    label="channel manifest",
                )


if __name__ == "__main__":
    unittest.main()
