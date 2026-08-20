from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from contextlib import contextmanager
from copy import deepcopy
from pathlib import Path

from validation.general_sem_rank0_packaged_acceptance import (
    ContractError,
    DEFAULT_CONTRACT,
    DEFAULT_REGISTRY,
    canonical_sha256,
    load_json,
    _package_fingerprint,
    qualification_contract_authorities,
    unified_rank0_source_receipt,
    validate_contract,
    validate_cell_report,
    validate_report,
    variant_canonical_authority,
)


ROOT = Path(__file__).resolve().parents[1]


def _artifact(path: Path) -> dict[str, object]:
    return {
        "kind": "acceptance_log",
        "path": path.relative_to(ROOT).as_posix(),
        "size": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }


def _write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


@contextmanager
def _build_report():
    contract = load_json(DEFAULT_CONTRACT)
    context = validate_contract(contract, load_json(DEFAULT_REGISTRY))
    results_root = ROOT / "validation/results"
    with tempfile.TemporaryDirectory(
        prefix="rank0-packaged-contract-test-", dir=results_root
    ) as directory_value:
        directory = Path(directory_value)
        package_identities = []
        portable_payload = b"prefix__TAURI_BUNDLE_TYPE_VAR_UNKsuffix"
        installed_payload = b"prefix__TAURI_BUNDLE_TYPE_VAR_NSSsuffix"
        for package in ("installed", "portable"):
            executable = directory / f"{package}.exe"
            payload = installed_payload if package == "installed" else portable_payload
            executable.write_bytes(payload)
            package_identities.append(
                {
                    "package_kind": package,
                    "resolved_path": str(executable.resolve()),
                    "size": executable.stat().st_size,
                    "sha256": hashlib.sha256(payload).hexdigest(),
                    "product_version": "3.0.0-test",
                    "file_version": "3.0.0.0",
                    "provenance": {
                        "evidence_kind": "windows_pe_package_identity_v1",
                        "file_identity_source": "resolved_path_size_sha256",
                        "version_identity_source": "System.Diagnostics.FileVersionInfo",
                    },
                }
            )
        package_by_kind = {row["package_kind"]: row for row in package_identities}
        results = []
        for package in context["packages"]:
            for variant in context["variants"]:
                evidence_dir = directory / package / variant["variant_id"]
                reference = {
                    "registry_schema_version": variant["reference"][0],
                    "capability_id": variant["reference"][1],
                    "cell_id": variant["reference"][2],
                    "capability_version": variant["reference"][3],
                }
                run_id = f"run-{package}-{variant['variant_id']}"
                document_id = f"document-{package}-{variant['variant_id']}"
                authority = variant_canonical_authority(variant)
                primary_reference = {
                    "registry_schema_version": authority["primary"][0],
                    "capability_id": authority["primary"][1],
                    "cell_id": authority["primary"][2],
                    "capability_version": authority["primary"][3],
                }
                supplemental_reference = (
                    {
                        "registry_schema_version": authority["supplemental"][0],
                        "capability_id": authority["supplemental"][1],
                        "cell_id": authority["supplemental"][2],
                        "capability_version": authority["supplemental"][3],
                    }
                    if authority["supplemental"] is not None
                    else None
                )
                project_path = evidence_dir / "project.qpls"
                project_path.parent.mkdir(parents=True, exist_ok=True)
                project_path.write_bytes(b"schema6-archive")
                canonical_document = {
                    "document_id": document_id,
                    "provenance": {
                        "run_id": run_id,
                        "capability_cell": primary_reference,
                        "method_version": authority["method_version"],
                    },
                    "capability_cells": [
                        primary_reference,
                        *(
                            [supplemental_reference]
                            if supplemental_reference is not None
                            else []
                        ),
                    ],
                    **(
                        {
                            "general_sem_results": {
                                "inference_receipt": {
                                    "capability_cell": supplemental_reference,
                                    "method_version": authority["method_version"],
                                }
                            }
                        }
                        if supplemental_reference is not None
                        else {}
                    ),
                }
                export_rows = []
                for format_id in ("csv", "xlsx", "html", "pdf", "svg", "png"):
                    export_path = evidence_dir / f"result.{format_id}"
                    export_path.write_bytes(f"{format_id}:{run_id}".encode())
                    descriptor = _artifact(export_path)
                    export_rows.append(
                        {
                            "format": format_id,
                            "path": descriptor["path"],
                            "size": descriptor["size"],
                            "sha256": descriptor["sha256"],
                            "semantic_readback": {
                                "schema_version": 1,
                                "evidence_kind": "general_sem_rank0_export_semantic_readback",
                                "format": format_id,
                                "document_id": document_id,
                                "run_id": run_id,
                                "method_version": authority["method_version"],
                                "dataset_fingerprint": "d" * 64,
                                "semantic_sha256": "e" * 64,
                                "table_ids": []
                                if format_id in {"svg", "png"}
                                else ["effects"],
                                "chart_ids": ["chart"]
                                if format_id in {"svg", "png"}
                                else [],
                                "canonical_values_sha256": "f" * 64,
                                "rendered_surface_match": True,
                                "canonical_match": True,
                                "passed": True,
                            },
                        }
                    )
                documents = {
                    "run_trace": {
                        "schema_version": 1,
                        "evidence_kind": "general_sem_rank0_run_trace",
                        "package_kind": package,
                        "variant_id": variant["variant_id"],
                        "capability_reference": reference,
                        "offline": True,
                        "offline_observations": [
                            {
                                "phase": "execute" if index == 0 else "reopen",
                                "scale_percent": (100, 100, 125, 150, 200)[index],
                                "observed_request_count": 3,
                                "external_request_count": 0,
                                "origins": ["http://ipc.localhost"],
                                "external_requests": [],
                                "passed": True,
                            }
                            for index in range(5)
                        ],
                        "steps": {check: True for check in variant["required_checks"]},
                        "run_id": run_id,
                        "document_id": document_id,
                        "project_archive": {
                            "path": project_path.relative_to(ROOT).as_posix(),
                            "size": project_path.stat().st_size,
                            "sha256": hashlib.sha256(
                                project_path.read_bytes()
                            ).hexdigest(),
                        },
                        "cancellation_observation": {
                            "terminal_latency_seconds": 0.25,
                            "terminal_state": "cancelled",
                            "job_completed_before_cancel": False,
                            "no_partial_visible_result": True,
                            "no_partial_committed_result": True,
                            "archive_unchanged": True,
                            "exact_same_settings_retry": True,
                            "archive_before": {
                                "byte_length": project_path.stat().st_size,
                                "sha256": hashlib.sha256(
                                    project_path.read_bytes()
                                ).hexdigest(),
                                "canonical_result_attachment_count": 0,
                            },
                            "archive_after": {
                                "byte_length": project_path.stat().st_size,
                                "sha256": hashlib.sha256(
                                    project_path.read_bytes()
                                ).hexdigest(),
                                "canonical_result_attachment_count": 0,
                            },
                        },
                        "export_cancellation_observation": {
                            "ui_control_cancellations": [
                                {
                                    "format": format_id,
                                    "destination_path": (
                                        evidence_dir
                                        / f"ui-cancelled-export.{format_id}"
                                    )
                                    .relative_to(ROOT)
                                    .as_posix(),
                                    "terminal_latency_seconds": 0.2,
                                    "terminal_state": "cancelled",
                                    "cancel_control_activated": True,
                                    "native_dialog_observed": False,
                                    "no_partial_file": True,
                                    "temp_files_unchanged": True,
                                }
                                for format_id in ("csv", "xlsx", "png")
                            ],
                            "save_dialog_destination_path": (
                                evidence_dir / "cancelled-export.csv"
                            )
                            .relative_to(ROOT)
                            .as_posix(),
                            "save_dialog_cancelled": True,
                            "semantic_readback_completed": True,
                            "save_dialog_no_partial_file": True,
                        },
                    },
                    "canonical_result": {
                        "schema_version": 1,
                        "evidence_kind": "general_sem_rank0_canonical_result",
                        "package_kind": package,
                        "variant_id": variant["variant_id"],
                        "capability_reference": reference,
                        "primary_capability_reference": primary_reference,
                        "supplemental_capability_reference": supplemental_reference,
                        "method_version": authority["method_version"],
                        "document_id": document_id,
                        "run_id": run_id,
                        "canonical_document_sha256": canonical_sha256(
                            canonical_document
                        ),
                        "canonical_document": canonical_document,
                    },
                    "exported_files_manifest": {
                        "schema_version": 1,
                        "evidence_kind": "general_sem_rank0_exported_files_manifest",
                        "package_kind": package,
                        "variant_id": variant["variant_id"],
                        "run_id": run_id,
                        "document_id": document_id,
                        "files": export_rows,
                    },
                    "accessibility_snapshot": {
                        "schema_version": 1,
                        "evidence_kind": "general_sem_rank0_accessibility_snapshot",
                        "package_kind": package,
                        "variant_id": variant["variant_id"],
                        "scales": [100, 125, 150, 200],
                        "viewports": ["1024x700", "1280x720", "1440x900"],
                        "cells": [
                            {
                                "scale_percent": scale,
                                "viewport": viewport,
                                "origin": "http://tauri.localhost",
                                "tauri_runtime": True,
                                "surface": "model",
                                "device_pixel_ratio": scale / 100,
                                "actual_client_width": int(viewport.split("x")[0]),
                                "actual_client_height": int(viewport.split("x")[1]),
                                "no_horizontal_overflow": True,
                                "table_count": 1,
                                "accessible_table_count": 1,
                                "chart_count": 1,
                                "accessible_chart_count": 1,
                                "keyboard_distinct_targets": 4,
                                "keyboard_reached_interactive_control": True,
                                "passed": True,
                            }
                            for scale in (100, 125, 150, 200)
                            for viewport in ("1024x700", "1280x720", "1440x900")
                        ],
                        "keyboard_navigation": True,
                        "accessible_table_and_chart": True,
                        "passed": True,
                    },
                    "process_cleanup_trace": {
                        "schema_version": 1,
                        "evidence_kind": "general_sem_rank0_process_cleanup_trace",
                        "package_kind": package,
                        "variant_id": variant["variant_id"],
                        "sessions": [
                            {
                                "session_id": "primary"
                                if index == 0
                                else f"scale_{(100, 125, 150, 200)[index - 1]}",
                                "phase": "execute" if index == 0 else "reopen",
                                "scale_percent": 100
                                if index == 0
                                else (100, 125, 150, 200)[index - 1],
                                "launched_pid": 1000 + index,
                                "launched_executable_path": package_by_kind[package][
                                    "resolved_path"
                                ],
                                "launched_executable_size": package_by_kind[package][
                                    "size"
                                ],
                                "launched_executable_sha256": package_by_kind[package][
                                    "sha256"
                                ],
                                "graceful_exit_confirmed": True,
                                "forced_termination": False,
                                "lingering_pids": [],
                                "cdp_endpoint_closed": True,
                                "passed": True,
                            }
                            for index in range(5)
                        ],
                        "orphan_process_ids": [],
                        "temporary_or_partial_files": [],
                        "passed": True,
                    },
                    "close_reopen_trace": {
                        "schema_version": 1,
                        "evidence_kind": "general_sem_rank0_close_reopen_trace",
                        "package_kind": package,
                        "variant_id": variant["variant_id"],
                        "project_archive_sha256": hashlib.sha256(
                            project_path.read_bytes()
                        ).hexdigest(),
                        "run_id": run_id,
                        "document_id": document_id,
                        "primary_pid": 1000,
                        "reopen_sessions": [
                            {
                                "scale_percent": scale,
                                "process_id": 1001 + index,
                                "run_id": run_id,
                                "document_id": document_id,
                                "project_archive_sha256": hashlib.sha256(
                                    project_path.read_bytes()
                                ).hexdigest(),
                                "closed": True,
                                "passed": True,
                            }
                            for index, scale in enumerate((100, 125, 150, 200))
                        ],
                        "passed": True,
                    },
                }
                artifacts = []
                for kind, document in documents.items():
                    path = evidence_dir / f"{kind}.json"
                    _write_json(path, document)
                    descriptor = _artifact(path)
                    descriptor["kind"] = kind
                    artifacts.append(descriptor)
                results.append(
                    {
                        "package_kind": package,
                        "variant_id": variant["variant_id"],
                        "capability_reference": reference,
                        "offline": True,
                        "fresh_process_reopen": True,
                        "checks": {check: True for check in variant["required_checks"]},
                        "artifacts": artifacts,
                    }
                )
        yield (
            context,
            {
                "schema_version": 1,
                "report_kind": "quickpls_general_sem_rank0_packaged_acceptance",
                "contract_id": context["contract_id"],
                "contract_version": context["contract_version"],
                "contract_sha256": canonical_sha256(contract),
                "build_fingerprint": package_identities[1]["sha256"],
                "package_set_fingerprint": _package_fingerprint(package_identities),
                "package_identities": package_identities,
                "hardware_fingerprint": {
                    "os": "windows_11",
                    "architecture": "x86_64",
                    "cpu": "Fixture CPU",
                    "physical_cores": 6,
                    "logical_cores": 12,
                    "memory_bytes": 16 * 1024**3,
                },
                "source_receipt": unified_rank0_source_receipt(ROOT),
                "qualification_contracts": qualification_contract_authorities(
                    context, ROOT
                ),
                "generated_at_utc": "2026-08-19T00:00:00Z",
                "results": results,
            },
        )


_REPORT_MANAGER = None
_REPORT_VALUE = None


@contextmanager
def _report():
    global _REPORT_MANAGER, _REPORT_VALUE
    if _REPORT_VALUE is None:
        _REPORT_MANAGER = _build_report()
        _REPORT_VALUE = _REPORT_MANAGER.__enter__()
    context, report = _REPORT_VALUE
    yield context, deepcopy(report)


def tearDownModule() -> None:
    global _REPORT_MANAGER, _REPORT_VALUE
    if _REPORT_MANAGER is not None:
        _REPORT_MANAGER.__exit__(None, None, None)
    _REPORT_MANAGER = None
    _REPORT_VALUE = None


class GeneralSemRank0PackagedAcceptanceTests(unittest.TestCase):
    def test_contract_resolves_exact_four_cells_and_two_packages(self) -> None:
        context = validate_contract(
            load_json(DEFAULT_CONTRACT), load_json(DEFAULT_REGISTRY)
        )
        self.assertEqual(context["packages"], ("installed", "portable"))
        self.assertEqual(
            [row["variant_id"] for row in context["variants"]],
            [
                "mediation_point",
                "multiple_mediation_bootstrap",
                "multiple_two_way_moderation_point",
                "multiple_two_way_moderation_bootstrap",
            ],
        )
        self.assertEqual(
            [row["bootstrap"] for row in context["variants"]],
            [False, True, False, True],
        )

    def test_complete_report_requires_exact_eight_result_matrix(self) -> None:
        with _report() as (context, report):
            result = validate_report(report, context, ROOT, require_standard=False)
            self.assertIs(result["passed"], True)
            self.assertEqual(result["result_count"], 8)

    def test_report_fails_closed_on_incomplete_or_tampered_evidence(self) -> None:
        for mutation in (
            "missing_result",
            "failed_check",
            "wrong_cell",
            "not_offline",
            "same_process_reopen",
            "stale_artifact",
        ):
            with self.subTest(mutation=mutation):
                with _report() as (context, original):
                    report = deepcopy(original)
                    first = report["results"][0]
                    if mutation == "missing_result":
                        report["results"].pop()
                    elif mutation == "failed_check":
                        first["checks"][next(iter(first["checks"]))] = False
                    elif mutation == "wrong_cell":
                        first["capability_reference"]["cell_id"] = "qpls3.pls.algorithm"
                    elif mutation == "not_offline":
                        first["offline"] = False
                    elif mutation == "same_process_reopen":
                        first["fresh_process_reopen"] = False
                    else:
                        first["artifacts"][0]["sha256"] = "0" * 64
                    with self.assertRaises(ContractError):
                        validate_report(report, context, ROOT, require_standard=False)

    def test_standard_gate_remains_closed_before_atomic_registry_promotion(
        self,
    ) -> None:
        with _report() as (context, report):
            with self.assertRaisesRegex(
                ContractError, "not release-qualified Standard"
            ):
                validate_report(report, context, ROOT, require_standard=True)

    def test_common_build_package_hardware_source_and_contract_authorities_fail_closed(
        self,
    ) -> None:
        for mutation in (
            "build",
            "package_set",
            "hardware",
            "source",
            "qualification_contract",
        ):
            with self.subTest(mutation=mutation):
                with _report() as (context, report):
                    if mutation == "build":
                        report["build_fingerprint"] = "0" * 64
                    elif mutation == "package_set":
                        report["package_set_fingerprint"] = "0" * 64
                    elif mutation == "hardware":
                        report["hardware_fingerprint"]["cpu"] = ""
                    elif mutation == "source":
                        report["source_receipt"]["source_set_sha256"] = "0" * 64
                    else:
                        report["qualification_contracts"][0][
                            "qualification_contract_sha256"
                        ] = "0" * 64
                    with self.assertRaises(ContractError):
                        validate_report(report, context, ROOT, require_standard=False)

    def test_cell_validator_ignores_unrelated_cell_failure_but_not_target_failure(
        self,
    ) -> None:
        with _report() as (context, original):
            report = deepcopy(original)
            target = report["results"][0]["capability_reference"]
            unrelated = report["results"][-1]
            unrelated["checks"][next(iter(unrelated["checks"]))] = False
            report["qualification_contracts"][-1]["qualification_contract_sha256"] = (
                "0" * 64
            )
            accepted = validate_cell_report(
                report,
                context,
                ROOT,
                capability_reference=target,
                require_standard=False,
            )
            self.assertIs(accepted["passed"], True)
            self.assertEqual(accepted["result_count"], 2)
            target_portable = next(
                row
                for row in report["results"]
                if row["package_kind"] == "portable"
                and row["capability_reference"] == target
            )
            target_portable["checks"][next(iter(target_portable["checks"]))] = False
            with self.assertRaisesRegex(ContractError, "failed required check"):
                validate_cell_report(
                    report,
                    context,
                    ROOT,
                    capability_reference=target,
                    require_standard=False,
                )


if __name__ == "__main__":
    unittest.main()
