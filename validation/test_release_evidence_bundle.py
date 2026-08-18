from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from validation import release_evidence_bundle as evidence


COMMIT = "a" * 40


def descriptor(path: str, payload: bytes) -> dict[str, object]:
    return {"path": path, "size": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}


def write_fixture(root: Path, runtime_payload: bytes = b"packaged-evidence") -> tuple[set[str], str]:
    source_path = "src/example.ts"
    runtime_path = "validation/results/screens/acceptance/example.png"
    source = root / source_path
    runtime = root / runtime_path
    source.parent.mkdir(parents=True)
    runtime.parent.mkdir(parents=True)
    source.write_bytes(b"tracked-source")
    runtime.write_bytes(runtime_payload)
    identity_path = root / "validation/results/method_factory/example_v1/packaged_acceptance.identity.json"
    identity_path.parent.mkdir(parents=True)
    identity = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "passed": True,
        "feature_id": "qpls3.example",
        "method_version": "example_v1",
        "catalogue_snapshot_date": "2026-08-12",
        "source_artifacts": [
            descriptor(source_path, source.read_bytes()),
            descriptor(runtime_path, runtime_payload),
        ],
    }
    identity_path.write_text(json.dumps(identity), encoding="utf-8")
    tracked = {source_path, identity_path.relative_to(root).as_posix()}
    return tracked, runtime_path


def clone_contract(source: Path, target: Path, tracked: set[str]) -> None:
    for relative in tracked:
        destination = target / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes((source / relative).read_bytes())


def rewrite_zip(source: Path, destination: Path, mutation) -> None:
    with zipfile.ZipFile(source, "r") as original, zipfile.ZipFile(destination, "w") as changed:
        for info in original.infolist():
            payload = original.read(info.filename)
            name, payload = mutation(info.filename, payload)
            changed.writestr(name, payload)


class ReleaseEvidenceBundleTests(unittest.TestCase):
    def test_pack_and_clean_checkout_restore_are_exact_and_non_claiming(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary) / "source"
            checkout = Path(temporary) / "checkout"
            workspace.mkdir()
            checkout.mkdir()
            tracked, runtime_path = write_fixture(workspace)
            bundle = Path(temporary) / "evidence.zip"

            packed = evidence.pack_bundle(
                bundle,
                root=workspace,
                source_commit=COMMIT,
                tracked_paths=tracked,
                generated_at="2026-08-14T00:00:00Z",
                require_clean=False,
            )
            clone_contract(workspace, checkout, tracked)
            restored = evidence.restore_bundle(
                bundle,
                root=checkout,
                expected_commit=COMMIT,
                tracked_paths=tracked,
            )

            self.assertTrue(packed["passed"])
            self.assertTrue(restored["passed"])
            self.assertFalse(restored["commercial_gate_satisfied"])
            self.assertFalse(restored["competitor_claims_authorized"])
            self.assertEqual((checkout / runtime_path).read_bytes(), b"packaged-evidence")

    def test_restore_rejects_payload_tamper_extra_entries_and_commit_drift(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "source"
            root.mkdir()
            tracked, runtime_path = write_fixture(root)
            bundle = Path(temporary) / "evidence.zip"
            evidence.pack_bundle(
                bundle,
                root=root,
                source_commit=COMMIT,
                tracked_paths=tracked,
                generated_at="2026-08-14T00:00:00Z",
                require_clean=False,
            )

            tampered = Path(temporary) / "tampered.zip"
            rewrite_zip(
                bundle,
                tampered,
                lambda name, payload: (name, b"tampered")
                if name == f"{evidence.PAYLOAD_PREFIX}{runtime_path}"
                else (name, payload),
            )
            checkout = Path(temporary) / "checkout-tamper"
            checkout.mkdir()
            clone_contract(root, checkout, tracked)
            with self.assertRaisesRegex(evidence.EvidenceBundleError, "size differs|hash differs"):
                evidence.restore_bundle(
                    tampered,
                    root=checkout,
                    expected_commit=COMMIT,
                    tracked_paths=tracked,
                )

            extra = Path(temporary) / "extra.zip"
            rewrite_zip(bundle, extra, lambda name, payload: (name, payload))
            with zipfile.ZipFile(extra, "a") as archive:
                archive.writestr("payload/unexpected.bin", b"unexpected")
            checkout = Path(temporary) / "checkout-extra"
            checkout.mkdir()
            clone_contract(root, checkout, tracked)
            with self.assertRaisesRegex(evidence.EvidenceBundleError, "unexpected entries"):
                evidence.restore_bundle(
                    extra,
                    root=checkout,
                    expected_commit=COMMIT,
                    tracked_paths=tracked,
                )

            checkout = Path(temporary) / "checkout-commit"
            checkout.mkdir()
            clone_contract(root, checkout, tracked)
            with self.assertRaisesRegex(evidence.EvidenceBundleError, "source commit"):
                evidence.restore_bundle(
                    bundle,
                    root=checkout,
                    expected_commit="b" * 40,
                    tracked_paths=tracked,
                )

    def test_restore_never_overwrites_existing_runtime_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "source"
            checkout = Path(temporary) / "checkout"
            root.mkdir()
            checkout.mkdir()
            tracked, runtime_path = write_fixture(root)
            bundle = Path(temporary) / "evidence.zip"
            evidence.pack_bundle(
                bundle,
                root=root,
                source_commit=COMMIT,
                tracked_paths=tracked,
                generated_at="2026-08-14T00:00:00Z",
                require_clean=False,
            )
            clone_contract(root, checkout, tracked)
            existing = checkout / runtime_path
            existing.parent.mkdir(parents=True)
            existing.write_bytes(b"do-not-overwrite")

            with self.assertRaisesRegex(evidence.EvidenceBundleError, "refusing to overwrite"):
                evidence.restore_bundle(
                    bundle,
                    root=checkout,
                    expected_commit=COMMIT,
                    tracked_paths=tracked,
                )
            self.assertEqual(existing.read_bytes(), b"do-not-overwrite")

    def test_pack_rejects_tracked_source_drift_and_expected_commit_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "source"
            root.mkdir()
            tracked, _ = write_fixture(root)
            bundle = Path(temporary) / "evidence.zip"

            (root / "src/example.ts").write_bytes(b"changed-after-identity")
            with self.assertRaisesRegex(evidence.EvidenceBundleError, "tracked source differs"):
                evidence.pack_bundle(
                    bundle,
                    root=root,
                    source_commit=COMMIT,
                    expected_commit=COMMIT,
                    tracked_paths=tracked,
                    require_clean=False,
                )

            (root / "src/example.ts").write_bytes(b"tracked-source")
            with self.assertRaisesRegex(evidence.EvidenceBundleError, "explicitly expected"):
                evidence.pack_bundle(
                    bundle,
                    root=root,
                    source_commit=COMMIT,
                    expected_commit="b" * 40,
                    tracked_paths=tracked,
                    require_clean=False,
                )

    def test_traversal_and_unapproved_runtime_roots_are_rejected(self) -> None:
        with self.assertRaises(evidence.EvidenceBundleError):
            evidence._relative_path("../escape", "test")
        with self.assertRaises(evidence.EvidenceBundleError):
            evidence._runtime_path("src/untracked.ts", "test")


if __name__ == "__main__":
    unittest.main()
