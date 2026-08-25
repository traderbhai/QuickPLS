#!/usr/bin/env python3
"""Contract tests for resumable heterogeneity qualification checkpoints."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import compare_multimod_heterogeneity_qualification_v2 as comparator
import multimod_heterogeneity_shards_v2 as shards


EXECUTABLE_SHA256 = "b" * 64
SOURCE_COMMIT = "a" * 40


def plan(rows: list[dict[str, object]]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "suite_id": shards.PLAN_SUITE_ID,
        "producer_suite_id": shards.PRODUCER_SUITE_ID,
        "scale": "qualification",
        "campaign_seed": 42,
        "metamorphism": "baseline",
        "sign_columns": None,
        "workers": 1,
        "fixture_observations": 400,
        "multiclass_point_fixture_plan": shards.MULTICLASS_POINT_FIXTURE_PLAN,
        "execution_contract": "one_cargo_build_then_dependency_aware_non_cargo_shards",
        "sentinel_shard_id": "sentinel",
        "aggregation_order": "plan_order",
        "shards": rows,
    }


def result(
    shard_id: str, kind: str, dependencies: list[str], index: int | None = None
) -> dict[str, object]:
    value: dict[str, object] = {
        "schema_version": 1,
        "suite_id": shards.SHARD_SUITE_ID,
        "producer_suite_id": shards.PRODUCER_SUITE_ID,
        "shard_id": shard_id,
        "scale": "qualification",
        "campaign_seed": 42,
        "metamorphism": "baseline",
        "sign_columns": None,
        "workers": 1,
        "fixture_observations": 400,
        "multiclass_point_fixture_plan": shards.MULTICLASS_POINT_FIXTURE_PLAN,
        "dependency_shard_ids": sorted(dependencies),
        "payload": {"kind": kind, "value": {"cell_id": shard_id}},
    }
    if index is not None:
        value["payload"]["index"] = index  # type: ignore[index]
    return value


def prepared_value(shard_id: str, dependencies: list[str]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "suite_id": shards.BOOTSTRAP_PREPARED_SUITE_ID,
        "producer_suite_id": shards.PRODUCER_SUITE_ID,
        "scientific_shard_id": shard_id,
        "scale": "qualification",
        "campaign_seed": 42,
        "metamorphism": "baseline",
        "sign_columns": None,
        "workers": 1,
        "fixture_observations": 400,
        "multiclass_point_fixture_plan": shards.MULTICLASS_POINT_FIXTURE_PLAN,
        "dependency_shard_ids": sorted(dependencies),
        "cell_id": "fimix-p0-fixed-k-bootstrap",
        "chunk_count": shards.BOOTSTRAP_CHUNK_COUNT,
        "requested_replicates": shards.BOOTSTRAP_REQUESTED_REPLICATES,
        "execution": {
            "method_version": "qpls.heterogeneity.raw-bootstrap-execution.v2",
            "execution_identity_sha256": "c" * 64,
            "reference": {
                "reference_identity_sha256": "d" * 64,
                "orchestrator_plan": {
                    "requested_replicates": shards.BOOTSTRAP_REQUESTED_REPLICATES,
                    "minimum_usable_fraction": 0.9,
                },
                "heterogeneity_plan": {
                    "requested_replicates": shards.BOOTSTRAP_REQUESTED_REPLICATES,
                    "minimum_usable_share": 0.9,
                },
            },
        },
    }


def cache_value(
    shard_id: str,
    dependencies: list[str],
    chunk_index: int,
    record_count: int,
    prior_record_count: int,
) -> dict[str, object]:
    expected_indices = list(
        range(
            chunk_index,
            shards.BOOTSTRAP_REQUESTED_REPLICATES,
            shards.BOOTSTRAP_CHUNK_COUNT,
        )
    )
    expected_count = len(expected_indices)
    completed = record_count == expected_count
    return {
        "schema_version": 1,
        "suite_id": shards.BOOTSTRAP_CACHE_SUITE_ID,
        "producer_suite_id": shards.PRODUCER_SUITE_ID,
        "scientific_shard_id": shard_id,
        "scale": "qualification",
        "campaign_seed": 42,
        "metamorphism": "baseline",
        "sign_columns": None,
        "workers": 1,
        "fixture_observations": 400,
        "multiclass_point_fixture_plan": shards.MULTICLASS_POINT_FIXTURE_PLAN,
        "dependency_shard_ids": sorted(dependencies),
        "cell_id": "fimix-p0-fixed-k-bootstrap",
        "prepared_execution_identity_sha256": "c" * 64,
        "chunk_index": chunk_index,
        "chunk_count": shards.BOOTSTRAP_CHUNK_COUNT,
        "requested_replicates": shards.BOOTSTRAP_REQUESTED_REPLICATES,
        "prior_record_count": prior_record_count,
        "record_count": record_count,
        "expected_record_count": expected_count,
        "completed": completed,
        "cache": {
            "cancelled": not completed,
            "shard": {
                "shard_index": chunk_index,
                "shard_count": shards.BOOTSTRAP_CHUNK_COUNT,
                "execution_identity_sha256": "e" * 64,
                "shard_identity_sha256": f"{chunk_index:064x}",
            },
            "records": [{"index": value} for value in expected_indices[:record_count]],
        },
    }


class HeterogeneityShardContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.shard_dir = self.root / "shards"
        self.shard_dir.mkdir()
        self.plan_path = self.root / "plan.json"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def seal(self, shard_id: str, value: dict[str, object]) -> None:
        temporary_result = self.root / f"{shard_id}.tmp.json"
        shards.atomic_json(temporary_result, value)
        shards.seal_checkpoint(
            types.SimpleNamespace(
                plan=self.plan_path,
                shard_dir=self.shard_dir,
                shard_id=shard_id,
                temporary_result=temporary_result,
                executable_sha256=EXECUTABLE_SHA256,
                source_commit=SOURCE_COMMIT,
            )
        )

    def prepare_bootstrap_checkpoint(self) -> tuple[str, list[str]]:
        rows = shards.expected_shard_specs("qualification")
        shards.atomic_json(self.plan_path, plan(rows))
        self.seal("sentinel", result("sentinel", "sentinel", []))
        self.seal(
            "fimix-recovery-00",
            result("fimix-recovery-00", "fimix_recovery", ["sentinel"], 0),
        )
        shard_id = "bootstrap-fimix-p0"
        dependencies = ["sentinel", "fimix-recovery-00"]
        temporary = self.root / "prepared.tmp.json"
        shards.atomic_json(temporary, prepared_value(shard_id, dependencies))
        shards.seal_bootstrap_prepared(
            types.SimpleNamespace(
                plan=self.plan_path,
                shard_dir=self.shard_dir,
                shard_id=shard_id,
                temporary_prepared=temporary,
                executable_sha256=EXECUTABLE_SHA256,
                source_commit=SOURCE_COMMIT,
            )
        )
        return shard_id, dependencies

    def seal_bootstrap_cache_value(
        self, shard_id: str, chunk_index: int, value: dict[str, object]
    ) -> None:
        temporary = self.root / f"cache-{chunk_index}.tmp.json"
        shards.atomic_json(temporary, value)
        shards.seal_bootstrap_cache(
            types.SimpleNamespace(
                plan=self.plan_path,
                shard_dir=self.shard_dir,
                shard_id=shard_id,
                chunk_index=chunk_index,
                temporary_cache=temporary,
                executable_sha256=EXECUTABLE_SHA256,
                source_commit=SOURCE_COMMIT,
            )
        )

    def test_sealed_checkpoint_rejects_result_tampering(self) -> None:
        rows = shards.expected_shard_specs("qualification")
        shards.atomic_json(self.plan_path, plan(rows))
        self.seal("sentinel", result("sentinel", "sentinel", []))
        shards.verify_checkpoint(
            self.plan_path,
            self.shard_dir,
            "sentinel",
            EXECUTABLE_SHA256,
            SOURCE_COMMIT,
        )
        altered = json.loads((self.shard_dir / "sentinel.json").read_text(encoding="utf-8"))
        altered["campaign_seed"] = 43
        shards.atomic_json(self.shard_dir / "sentinel.json", altered)
        with self.assertRaises(shards.ContractError):
            shards.verify_checkpoint(
                self.plan_path,
                self.shard_dir,
                "sentinel",
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
            )

    def test_child_receipt_is_bound_to_exact_dependency_receipt(self) -> None:
        rows = shards.expected_shard_specs("qualification")
        shards.atomic_json(self.plan_path, plan(rows))
        self.seal("sentinel", result("sentinel", "sentinel", []))
        self.seal(
            "fimix-recovery-00",
            result("fimix-recovery-00", "fimix_recovery", ["sentinel"], 0),
        )
        dependency_receipt = json.loads(
            (self.shard_dir / "sentinel.receipt.json").read_text(encoding="utf-8")
        )
        dependency_receipt["result_sha256"] = "0" * 64
        shards.atomic_json(self.shard_dir / "sentinel.receipt.json", dependency_receipt)
        with self.assertRaises(shards.ContractError):
            shards.verify_checkpoint(
                self.plan_path,
                self.shard_dir,
                "fimix-recovery-00",
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
            )

    def test_plan_rejects_forward_dependency(self) -> None:
        rows = shards.expected_shard_specs("qualification")
        rows[1]["dependencies"] = ["fimix-recovery-01"]
        shards.atomic_json(self.plan_path, plan(rows))
        with self.assertRaises(shards.ContractError):
            shards.validated_plan(self.plan_path)

    def test_bootstrap_prepare_and_cache_tampering_fail_closed(self) -> None:
        shard_id, dependencies = self.prepare_bootstrap_checkpoint()
        shards.verify_bootstrap_prepared(
            self.plan_path,
            self.shard_dir,
            shard_id,
            EXECUTABLE_SHA256,
            SOURCE_COMMIT,
        )
        self.seal_bootstrap_cache_value(
            shard_id,
            0,
            cache_value(shard_id, dependencies, 0, 1, 0),
        )
        cache_path = shards.bootstrap_cache_path(self.shard_dir, shard_id, 0)
        altered = json.loads(cache_path.read_text(encoding="utf-8"))
        altered["record_count"] = 2
        shards.atomic_json(cache_path, altered)
        with self.assertRaises(shards.ContractError):
            shards.verify_bootstrap_cache(
                self.plan_path,
                self.shard_dir,
                shard_id,
                0,
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
            )

    def test_bootstrap_prepared_tampering_fails_closed(self) -> None:
        shard_id, _ = self.prepare_bootstrap_checkpoint()
        prepared_path = shards.bootstrap_prepared_path(self.shard_dir, shard_id)
        altered = json.loads(prepared_path.read_text(encoding="utf-8"))
        altered["execution"]["execution_identity_sha256"] = "f" * 64
        shards.atomic_json(prepared_path, altered)
        with self.assertRaises(shards.ContractError):
            shards.verify_bootstrap_prepared(
                self.plan_path,
                self.shard_dir,
                shard_id,
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
            )

    def test_bootstrap_current_pointer_tampering_fails_closed(self) -> None:
        shard_id, _ = self.prepare_bootstrap_checkpoint()
        pointer_path = shards.bootstrap_prepared_pointer_path(
            self.shard_dir, shard_id
        )
        pointer = shards.read_json(pointer_path)
        pointer["payload_sha256"] = "0" * 64
        shards.atomic_json(pointer_path, pointer)
        with self.assertRaisesRegex(shards.ContractError, "incomplete or altered"):
            shards.verify_bootstrap_prepared(
                self.plan_path,
                self.shard_dir,
                shard_id,
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
            )

    def test_bootstrap_resume_requires_strict_verified_progress(self) -> None:
        shard_id, dependencies = self.prepare_bootstrap_checkpoint()
        self.seal_bootstrap_cache_value(
            shard_id,
            0,
            cache_value(shard_id, dependencies, 0, 1, 0),
        )
        with self.assertRaisesRegex(shards.ContractError, "zero verified record progress"):
            self.seal_bootstrap_cache_value(
                shard_id,
                0,
                cache_value(shard_id, dependencies, 0, 1, 1),
            )
        self.seal_bootstrap_cache_value(
            shard_id,
            0,
            cache_value(shard_id, dependencies, 0, 2, 1),
        )
        resumed = shards.verify_bootstrap_cache(
            self.plan_path,
            self.shard_dir,
            shard_id,
            0,
            EXECUTABLE_SHA256,
            SOURCE_COMMIT,
        )
        self.assertEqual(resumed["record_count"], 2)

    def test_orphan_generations_do_not_displace_last_verified_checkpoint(self) -> None:
        shard_id, dependencies = self.prepare_bootstrap_checkpoint()
        self.seal_bootstrap_cache_value(
            shard_id,
            0,
            cache_value(shard_id, dependencies, 0, 1, 0),
        )
        prior_path = shards.bootstrap_cache_path(self.shard_dir, shard_id, 0)
        prior_pointer = shards.read_json(
            shards.bootstrap_cache_pointer_path(self.shard_dir, shard_id, 0)
        )
        cell_dir = shards.bootstrap_cell_dir(self.shard_dir, shard_id)
        orphan_generation = "f" * 32
        shards.atomic_json(
            cell_dir / f"cache-000-of-100.g-{orphan_generation}.json",
            cache_value(shard_id, dependencies, 0, 2, 1),
        )
        shards.atomic_json(
            cell_dir / f"cache-000-of-100.g-{orphan_generation}.receipt.json",
            {"orphaned_before_pointer_switch": True},
        )

        retained = shards.verify_bootstrap_cache(
            self.plan_path,
            self.shard_dir,
            shard_id,
            0,
            EXECUTABLE_SHA256,
            SOURCE_COMMIT,
        )
        self.assertEqual(retained["record_count"], 1)
        self.assertEqual(
            shards.read_json(
                shards.bootstrap_cache_pointer_path(self.shard_dir, shard_id, 0)
            ),
            prior_pointer,
        )
        self.assertEqual(shards.bootstrap_cache_path(self.shard_dir, shard_id, 0), prior_path)

        self.seal_bootstrap_cache_value(
            shard_id,
            0,
            cache_value(shard_id, dependencies, 0, 2, 1),
        )
        self.assertNotEqual(
            shards.bootstrap_cache_path(self.shard_dir, shard_id, 0), prior_path
        )
        self.assertEqual(
            shards.verify_bootstrap_cache(
                self.plan_path,
                self.shard_dir,
                shard_id,
                0,
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
            )["record_count"],
            2,
        )

    def test_bootstrap_inventory_requires_exact_complete_100_cache_set(self) -> None:
        shard_id, dependencies = self.prepare_bootstrap_checkpoint()
        _, by_id = shards.validated_plan(self.plan_path)
        with self.assertRaises(shards.ContractError):
            shards.complete_bootstrap_inventory(
                self.plan_path,
                self.shard_dir,
                by_id[shard_id],
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
            )
        for chunk_index in range(shards.BOOTSTRAP_CHUNK_COUNT):
            self.seal_bootstrap_cache_value(
                shard_id,
                chunk_index,
                cache_value(shard_id, dependencies, chunk_index, 5, 0),
            )
        inventory = shards.complete_bootstrap_inventory(
            self.plan_path,
            self.shard_dir,
            by_id[shard_id],
            EXECUTABLE_SHA256,
            SOURCE_COMMIT,
        )
        self.assertEqual(inventory["chunk_count"], 100)
        self.assertEqual(len(inventory["caches"]), 100)
        inventory_path = shards.bootstrap_cache_inventory_path(
            self.shard_dir, shard_id
        )
        shards.write_bootstrap_inventory(
            types.SimpleNamespace(
                plan=self.plan_path,
                shard_dir=self.shard_dir,
                shard_id=shard_id,
                output=inventory_path,
                executable_sha256=EXECUTABLE_SHA256,
                source_commit=SOURCE_COMMIT,
            )
        )
        serialized = shards.read_json(inventory_path)
        self.assertEqual(serialized, inventory)
        for row in serialized["caches"]:
            self.assertFalse(Path(row["cache_file"]).is_absolute())
            self.assertEqual(Path(row["cache_file"]).parent, Path("."))
        self.assertLess(len(str(inventory_path)), 32767)
        mixed = shards.bootstrap_cache_path(self.shard_dir, shard_id, 99)
        value = json.loads(mixed.read_text(encoding="utf-8"))
        value["prepared_execution_identity_sha256"] = "f" * 64
        shards.atomic_json(mixed, value)
        with self.assertRaises(shards.ContractError):
            shards.complete_bootstrap_inventory(
                self.plan_path,
                self.shard_dir,
                by_id[shard_id],
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
            )

    def test_lean_k_matrix_keeps_three_point_discoveries_and_seven_bootstraps(self) -> None:
        rows = shards.expected_shard_specs("qualification")
        shard_ids = {row["shard_id"] for row in rows}
        for selected_k in range(3, 6):
            self.assertIn(f"pos-published-k{selected_k}-discovery", shard_ids)
            self.assertNotIn(f"pos-published-k{selected_k}-bootstrap", shard_ids)
        self.assertEqual(
            sum(row["resource_class"] == "bootstrap" for row in rows),
            7,
        )
        self.assertEqual(len(rows), 52)
        self.assertEqual(
            comparator.EXPECTED_BOOTSTRAP_CELL_IDS,
            frozenset(
                {
                    "fimix-p0-fixed-k-bootstrap",
                    "pos-published-p0-fixed-k-bootstrap",
                    "fimix-p2-fixed-k-bootstrap",
                    "pos-destination-p2-fixed-k-bootstrap",
                    "fimix-p23-fixed-k-bootstrap",
                    "pos-destination-p23-fixed-k-bootstrap",
                    "pos-p2-common-metric-failure-fixed-k-bootstrap",
                }
            ),
        )
        self.assertFalse(
            any(
                f"k{selected_k}-fixed-k-bootstrap" in check_id
                for selected_k in range(3, 6)
                for check_id in comparator.BOUND_GATE_CHECK_IDS
            )
        )
        self.assertNotIn(
            "heterogeneity.bootstrap.pos_published_p0_k2_through_k5_inventory",
            comparator.BOUND_GATE_CHECK_IDS,
        )

        value = plan(rows)
        self.assertEqual(
            value["multiclass_point_fixture_plan"],
            {
                "schema_version": 1,
                "plan_id": "qpls.multimod.heterogeneity.pos-published-p0-k3-k5-point-discovery.v1",
                "purpose": "published_p0_pos_candidate_point_discovery_only",
                "selected_k": [3, 4, 5],
                "observations_per_fixture": 120,
                "allocation": "row_mod_k_exactly_balanced",
                "bootstrap_evidence": "not_requested",
            },
        )

    def test_development_plan_remains_point_only_and_never_enters_bootstrap_transport(self) -> None:
        rows = shards.expected_shard_specs("development")
        self.assertEqual(len(rows), 22)
        self.assertFalse(any(row["resource_class"] == "bootstrap" for row in rows))
        development = plan(rows)
        development["scale"] = "development"
        shards.atomic_json(self.plan_path, development)
        validated, _ = shards.validated_plan(self.plan_path)
        self.assertEqual(validated["scale"], "development")

    def test_comparator_requires_exact_baseline_qualification_matrix(self) -> None:
        identity = {
            "scale": "qualification",
            "metamorphism": "baseline",
            "sign_columns": None,
            "workers": 1,
            "fixture_observations": 400,
            "multiclass_point_fixture_plan": shards.MULTICLASS_POINT_FIXTURE_PLAN,
            "campaign_seed": 42,
            "seed": 42,
        }
        self.assertTrue(comparator.is_baseline_qualification_matrix(identity))
        for key, replacement in (
            ("scale", "development"),
            ("metamorphism", "row_reverse"),
            ("sign_columns", "x1"),
            ("workers", 2),
            ("fixture_observations", 80),
            ("multiclass_point_fixture_plan", {}),
            ("campaign_seed", 43),
            ("seed", 43),
        ):
            with self.subTest(key=key):
                altered = dict(identity)
                altered[key] = replacement
                self.assertFalse(comparator.is_baseline_qualification_matrix(altered))

    def test_comparator_reconciles_frozen_bootstrap_failure_ledger(self) -> None:
        fit_detail = (
            "MultiMod statistical kernel failed: PLS-POS optimum was reproduced by "
            "1 starts; 2 required"
        )
        entries = [
            {
                "replicate_index": 0,
                "status": "usable",
                "failure_reason": None,
            },
            {
                "replicate_index": 1,
                "status": "fit_failed",
                "failure_reason": fit_detail,
            },
            {
                "replicate_index": 2,
                "status": "label_ambiguous",
                "failure_reason": "bootstrap class/segment label match was ambiguous",
            },
        ]
        ledger = {
            "failure_counts": {
                "heterogeneity.bootstrap.fit_failed": 1,
                "heterogeneity.bootstrap.label_ambiguous": 1,
            },
            "failures": [
                {
                    "replicate_index": 1,
                    "kind": "estimator_did_not_converge",
                    "stable_code": "heterogeneity.bootstrap.fit_failed",
                    "detail": fit_detail,
                },
                {
                    "replicate_index": 2,
                    "kind": "ambiguous_label_alignment",
                    "stable_code": "heterogeneity.bootstrap.label_ambiguous",
                    "detail": "bootstrap class/segment label match was ambiguous",
                },
            ],
        }

        reconciled, receipt = comparator.reconcile_bootstrap_failures(entries, ledger)
        self.assertTrue(reconciled, receipt)
        self.assertEqual(
            comparator.BOOTSTRAP_FAILURE_PUBLIC_CONTRACT["fit_failed"],
            (
                "heterogeneity.bootstrap.fit_failed",
                "estimator_did_not_converge",
            ),
        )

        mutations = (
            lambda value: value["failure_counts"].update(
                {"heterogeneity.bootstrap.fit_failed": 2}
            ),
            lambda value: value["failures"][0].update(
                {"stable_code": "heterogeneity.bootstrap.unstable_multistart"}
            ),
            lambda value: value["failures"][0].update(
                {"kind": "unstable_multistart"}
            ),
            lambda value: value["failures"][0].update({"detail": "altered"}),
            lambda value: value["failures"][0].update({"replicate_index": 99}),
        )
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                altered = json.loads(json.dumps(ledger))
                mutate(altered)
                self.assertFalse(
                    comparator.reconcile_bootstrap_failures(entries, altered)[0]
                )

    def test_comparator_failure_reconciliation_rejects_unknown_and_extra_rows(self) -> None:
        empty_ledger = {"failure_counts": {}, "failures": []}
        self.assertTrue(comparator.reconcile_bootstrap_failures([], empty_ledger)[0])
        self.assertFalse(
            comparator.reconcile_bootstrap_failures([], {"failure_counts": {}})[0]
        )

        unknown = [
            {
                "replicate_index": 0,
                "status": "unstable_multistart",
                "failure_reason": "not a frozen V2 ledger status",
            }
        ]
        self.assertFalse(
            comparator.reconcile_bootstrap_failures(unknown, empty_ledger)[0]
        )

        extra_public_row = {
            "failure_counts": {},
            "failures": [
                {
                    "replicate_index": 0,
                    "kind": "estimator_did_not_converge",
                    "stable_code": "heterogeneity.bootstrap.fit_failed",
                    "detail": "orphan public failure",
                }
            ],
        }
        self.assertFalse(
            comparator.reconcile_bootstrap_failures([], extra_public_row)[0]
        )

    def test_wrapper_freezes_baseline_caps_and_bounded_children(self) -> None:
        wrapper = Path(__file__).with_name(
            "run_multimod_heterogeneity_qualification_v2.ps1"
        ).read_text(encoding="utf-8")
        for fragment in (
            '[ValidateRange(1, 4)]',
            '[ValidateRange(1, 2)]',
            '[ValidateRange(60, 1800)]',
            '[ValidateRange(600, 6600)]',
            '[int]$OverallTimeoutSeconds = 6480',
            '[ValidateRange(1, 1500)]',
            'QPLS_MULTIMOD_METAMORPHISM_V1',
            'QPLS_MULTIMOD_WORKERS_V1',
            'QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1',
            'QPLS_MULTIMOD_SIGN_COLUMNS_V1',
            'git status --porcelain=v1 --untracked-files=all',
            'function Start-BoundedChild',
            'function Wait-BoundedChild',
            'function Stop-BoundedChild',
            'WaitForExit(10000)',
            '-Stage "cargo-build"',
            '-Stage "plan"',
            '-Stage "aggregate"',
            '-Stage "comparator"',
            '$job.TimeoutSeconds = [Math]::Min($job.TimeoutSeconds, $sentinelTimeoutSeconds)',
            '$bootstrapChunkCount = 100',
            '$active.Count -ge $MaxParallelBootstrapShards',
            '"--bootstrap-prepare"',
            '"--bootstrap-chunk"',
            '"--bootstrap-finalize"',
            '"--cache-inventory"',
            '"write-bootstrap-inventory"',
            '"seal-bootstrap-prepared"',
            '"seal-bootstrap-cache"',
            '$childElapsedSeconds -ge $Job.TimeoutSeconds',
            '$campaignElapsedAtExitSeconds -ge $workCutoffSeconds',
            '$workCutoffSeconds = [Math]::Min($OverallTimeoutSeconds, 6480)',
            'bootstrap_child_timeout',
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, wrapper)

        wait_start = wrapper.index("function Wait-BoundedChild")
        wait_end = wrapper.index("function Invoke-BoundedStage", wait_start)
        wait_source = wrapper[wait_start:wait_end]
        self.assertLess(
            wait_source.index("WaitForExit(10000)"),
            wait_source.index("$childElapsedSeconds -ge $Job.TimeoutSeconds"),
        )
        self.assertNotIn('"--cache"', wrapper)

        chunk_start = wrapper.index('elseif ($State.Phase -eq "chunk")')
        chunk_end = wrapper.index('elseif ($State.Phase -eq "finalize")', chunk_start)
        chunk_source = wrapper[chunk_start:chunk_end]
        self.assertIn("$cachePointer = Get-BootstrapCachePointerPath", chunk_source)
        self.assertLess(
            chunk_source.index("if (Test-Path -LiteralPath $cachePointer -PathType Leaf)"),
            chunk_source.index("$cachePath = Get-BootstrapCachePath"),
        )

        bindings = json.loads(
            Path(__file__)
            .with_name("multimod_gate_bindings_v1.json")
            .read_text(encoding="utf-8")
        )
        recovery_gate = next(
            gate for gate in bindings["gates"] if gate["gate_id"] == "fimix.recovery"
        )
        production_step = next(
            step
            for step in recovery_gate["steps"]
            if step["step_id"] == "heterogeneity_production_science"
        )
        self.assertEqual(production_step["maximum_seconds"], 6600)

        self.assertIn("$paths = @(\n        @(\n", wrapper)
        self.assertIn(
            ") | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }\n"
            "    )\n"
            "    if ($paths.Count -eq 0)",
            wrapper,
        )

        comparator_source = Path(comparator.__file__).read_text(encoding="utf-8")
        self.assertIn(
            '"producer_execution_receipt": report.get("shard_execution_receipt")',
            comparator_source,
        )

    def test_wait_bounded_child_rejects_an_already_exited_late_process(self) -> None:
        wrapper_path = Path(__file__).with_name(
            "run_multimod_heterogeneity_qualification_v2.ps1"
        )
        quote = lambda value: str(value).replace("'", "''")
        script = f"""
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    '{quote(wrapper_path)}', [ref]$tokens, [ref]$errors
)
if ($errors.Count -ne 0) {{ exit 10 }}
$names = @(
    'Get-RemainingBudgetSeconds', 'Start-BoundedChild', 'Save-ProcessLogs',
    'Stop-BoundedChild', 'Wait-BoundedChild'
)
foreach ($name in $names) {{
    $node = $ast.Find({{
        param($candidate)
        $candidate -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $candidate.Name -eq $name
    }}, $true)
    Invoke-Expression $node.Extent.Text
}}
$global:campaignStartedAtUtc = [DateTime]::UtcNow
$global:campaignClock = [System.Diagnostics.Stopwatch]::StartNew()
$global:workCutoffSeconds = 30
$job = Start-BoundedChild -Stage 'post-exit-deadline-contract' -FileName 'pwsh' `
    -Arguments @('-NoProfile', '-Command', 'Start-Sleep -Milliseconds 200') `
    -StdoutPath '{quote(self.root / "late.stdout.log")}' `
    -StderrPath '{quote(self.root / "late.stderr.log")}' -TimeoutSeconds 10
[void]$job.Process.WaitForExit(5000)
if (-not $job.Process.HasExited) {{ exit 13 }}
$job.TimeoutSeconds = 0.1
try {{
    [void](Wait-BoundedChild -Job $job)
    exit 11
}}
catch {{
    if ($_.Exception.Message -notlike '*exact bounded deadline*') {{
        [Console]::Error.WriteLine($_.Exception.Message)
        exit 12
    }}
    Write-Output 'late-exit-rejected'
}}
"""
        completed = subprocess.run(
            ["pwsh", "-NoProfile", "-Command", script],
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("late-exit-rejected", completed.stdout)

    def test_rust_producer_wires_prepare_chunk_and_global_finalize_apis(self) -> None:
        producer = (
            Path(__file__).resolve().parents[2]
            / "crates/qpls-runner/examples/multimod_heterogeneity_qualification_v2.rs"
        ).read_text(encoding="utf-8")
        for fragment in (
            "prepare_compiled_raw_pls_heterogeneity_bootstrap_v2(",
            "run_prepared_raw_pls_heterogeneity_bootstrap_shard_v2(",
            "finalize_prepared_raw_pls_heterogeneity_bootstrap_v2(",
            "const DEFAULT_BOOTSTRAP_CHUNK_COUNT: u32 = 100;",
            "const QUALIFICATION_BOOTSTRAP_DRAWS: u32 = 500;",
            "made zero verified record progress",
            "bootstrap cache inventory is missing, duplicate, or mixed",
            '"--cache-inventory"',
            "BOOTSTRAP_CACHE_INVENTORY_SUITE_ID",
            "validate_generation_pointer(",
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, producer)


if __name__ == "__main__":
    unittest.main()
