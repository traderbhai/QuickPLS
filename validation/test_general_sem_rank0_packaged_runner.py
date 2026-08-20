from __future__ import annotations

import hashlib
import json
import unittest

from validation.general_sem_rank0_packaged_acceptance import (
    DEFAULT_CONTRACT,
    DEFAULT_REGISTRY,
    ContractError,
    load_json,
    validate_contract,
    variant_canonical_authority,
)
from validation.general_sem_rank0_packaged_runner import (
    ROOT,
    _canonical_attachment,
    _normalize_cancellation,
    _normalize_export_cancellation,
    _normalize_offline_observation,
    package_set_fingerprint,
)


def _reference(value):
    return {
        "registry_schema_version": value[0],
        "capability_id": value[1],
        "cell_id": value[2],
        "capability_version": value[3],
    }


def _project(variant):
    authority = variant_canonical_authority(variant)
    primary = _reference(authority["primary"])
    supplemental = (
        _reference(authority["supplemental"])
        if authority["supplemental"] is not None
        else None
    )
    document = {
        "schema_version": 2,
        "document_id": "document-1",
        "provenance": {
            "run_id": "run-1",
            "capability_cell": primary,
            "method_version": authority["method_version"],
        },
        "capability_cells": [
            primary,
            *([supplemental] if supplemental is not None else []),
        ],
        **(
            {
                "general_sem_results": {
                    "inference_receipt": {
                        "capability_cell": supplemental,
                        "method_version": authority["method_version"],
                    }
                }
            }
            if supplemental is not None
            else {}
        ),
    }
    digest = hashlib.sha256(
        json.dumps(
            document,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
        ).encode()
    ).hexdigest()
    return {
        "canonical_result_documents": [
            {
                "run_id": "run-1",
                "document_id": "document-1",
                "canonical_document_sha256": digest,
                "canonical_document": document,
            }
        ]
    }


class GeneralSemRank0PackagedRunnerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.variants = validate_contract(
            load_json(DEFAULT_CONTRACT), load_json(DEFAULT_REGISTRY)
        )["variants"]

    def test_all_four_canonical_authorities_preserve_point_primary(self) -> None:
        for variant in self.variants:
            with self.subTest(variant=variant["variant_id"]):
                document, digest, authority = _canonical_attachment(
                    _project(variant),
                    run_id="run-1",
                    document_id="document-1",
                    variant=variant,
                )
                self.assertEqual(
                    document["provenance"]["capability_cell"],
                    _reference(authority["primary"]),
                )
                self.assertEqual(len(digest), 64)

    def test_bootstrap_rejects_supplemental_cell_as_primary(self) -> None:
        variant = next(row for row in self.variants if row["bootstrap"])
        project = _project(variant)
        document = project["canonical_result_documents"][0]["canonical_document"]
        authority = variant_canonical_authority(variant)
        document["provenance"]["capability_cell"] = _reference(
            authority["supplemental"]
        )
        with self.assertRaisesRegex(ContractError, "point-primary"):
            _canonical_attachment(
                project,
                run_id="run-1",
                document_id="document-1",
                variant=variant,
            )

    def test_bootstrap_rejects_missing_supplemental_receipt(self) -> None:
        variant = next(row for row in self.variants if row["bootstrap"])
        project = _project(variant)
        document = project["canonical_result_documents"][0]["canonical_document"]
        del document["general_sem_results"]["inference_receipt"]
        with self.assertRaisesRegex(ContractError, "inference receipt"):
            _canonical_attachment(
                project,
                run_id="run-1",
                document_id="document-1",
                variant=variant,
            )

    def test_point_mediation_rejects_capability_version_as_outer_method(self) -> None:
        variant = next(
            row for row in self.variants if row["variant_id"] == "mediation_point"
        )
        project = _project(variant)
        document = project["canonical_result_documents"][0]["canonical_document"]
        document["provenance"]["method_version"] = variant["reference"][3]
        with self.assertRaisesRegex(ContractError, "method_version"):
            _canonical_attachment(
                project,
                run_id="run-1",
                document_id="document-1",
                variant=variant,
            )

    def test_cancellation_binds_latency_archive_bytes_and_attachment_count(self) -> None:
        archive = {
            "schema_version": 1,
            "evidence_kind": "general_sem_rank0_schema6_archive_identity",
            "archive_path": str((ROOT / "validation/results/never-created.qpls").resolve()),
            "byte_length": 123,
            "sha256": "a" * 64,
            "project_schema_version": 6,
            "sem_generation": "general_sem_v1",
            "canonical_result_attachment_count": 0,
        }
        value = {
            "terminalLatencySeconds": 0.25,
            "terminalState": "cancelled",
            "jobCompletedBeforeCancel": False,
            "noPartialVisibleResult": True,
            "noPartialCommittedResult": True,
            "archiveUnchanged": True,
            "exactSameSettingsRetry": True,
            "visibleResultCountBefore": 0,
            "visibleResultCountAfter": 0,
            "committedResultActionCount": 0,
            "archiveBefore": archive,
            "archiveAfter": dict(archive),
            "settingsBefore": {"bootstrap": True, "samples": "500"},
            "settingsRetry": {"bootstrap": True, "samples": "500"},
        }
        normalized = _normalize_cancellation(value, True)
        self.assertEqual(normalized["terminal_latency_seconds"], 0.25)

        value["archiveAfter"] = {**archive, "sha256": "b" * 64}
        with self.assertRaisesRegex(ContractError, "archive bytes"):
            _normalize_cancellation(value, True)

    def test_export_cancellation_requires_owned_dialog_receipt_and_no_file(self) -> None:
        save_destination = (
            ROOT / "validation/results/rank0-cancelled-export-never-created.csv"
        ).resolve()
        self.assertFalse(save_destination.exists())
        value = {
            "saveDialog": {
                "format": "csv",
                "destinationPath": str(save_destination),
                "nativeDialogCancelled": True,
                "semanticReadbackCompleted": True,
                "destinationExistedAfter": False,
                "noPartialFile": True,
                "publication": {
                    "event": "complete",
                    "passed": True,
                    "mode": "save-cancel",
                    "file": {
                        "path": str(save_destination),
                        "exists": False,
                        "cancelledBeforePublication": True,
                    },
                },
            },
        }
        normalized = _normalize_export_cancellation(value, ROOT)
        self.assertIs(normalized["save_dialog_no_partial_file"], True)
        value["saveDialog"]["publication"]["file"]["exists"] = True
        with self.assertRaisesRegex(ContractError, "zero-file"):
            _normalize_export_cancellation(value, ROOT)

    def test_offline_observation_requires_positive_internal_and_zero_external_requests(self) -> None:
        value = {
            "passed": True,
            "observedRequestCount": 3,
            "externalRequestCount": 0,
            "origins": ["http://ipc.localhost", "http://tauri.localhost"],
            "externalRequests": [],
        }
        normalized = _normalize_offline_observation(
            value, phase="execute", scale_percent=100
        )
        self.assertEqual(normalized["observed_request_count"], 3)
        value["externalRequestCount"] = 1
        value["externalRequests"] = [{"url": "https://example.test"}]
        with self.assertRaisesRegex(ContractError, "zero-external-request"):
            _normalize_offline_observation(
                value, phase="execute", scale_percent=100
            )

    def test_package_fingerprint_binds_paths_bytes_versions_and_provenance(self) -> None:
        provenance = {
            "evidence_kind": "windows_pe_package_identity_v1",
            "file_identity_source": "resolved_path_size_sha256",
            "version_identity_source": "System.Diagnostics.FileVersionInfo",
        }
        packages = [
            {
                "package_kind": kind,
                "resolved_path": f"D:/packages/{kind}/quickpls.exe",
                "size": 10 + index,
                "sha256": str(index + 1) * 64,
                "product_version": "3.0.0",
                "file_version": "3.0.0.0",
                "provenance": provenance,
            }
            for index, kind in enumerate(("installed", "portable"))
        ]
        fingerprint = package_set_fingerprint(packages)
        self.assertEqual(len(fingerprint), 64)
        packages[1]["file_version"] = "3.0.0.1"
        self.assertNotEqual(package_set_fingerprint(packages), fingerprint)


if __name__ == "__main__":
    unittest.main()
