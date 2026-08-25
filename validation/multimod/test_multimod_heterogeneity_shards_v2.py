#!/usr/bin/env python3
"""Contract tests for resumable heterogeneity qualification checkpoints."""

from __future__ import annotations

import json
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
        "dependency_shard_ids": sorted(dependencies),
        "payload": {"kind": kind, "value": {"cell_id": shard_id}},
    }
    if index is not None:
        value["payload"]["index"] = index  # type: ignore[index]
    return value


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

    def test_comparator_requires_exact_baseline_qualification_matrix(self) -> None:
        identity = {
            "scale": "qualification",
            "metamorphism": "baseline",
            "sign_columns": None,
            "workers": 1,
            "fixture_observations": 400,
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
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, wrapper)

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


if __name__ == "__main__":
    unittest.main()
