#!/usr/bin/env python3
"""Contract tests for conditional/causal resumable qualification shards."""

from __future__ import annotations

import copy
import json
import re
import sys
import tempfile
import types
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import conditional_causal_shards_v1 as shards
import verify_conditional_causal_raw_qualification_v1 as verifier


SOURCE_COMMIT = "a" * 40


def plan(family: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "suite_id": shards.PLAN_SUITE_ID,
        "family": family,
        "producer_id": shards.PRODUCER_IDS[family],
        "scale": "qualification",
        "seed": 42,
        "metamorphism": "baseline",
        "sign_columns": None,
        "workers": 1,
        "execution_contract": "one_cargo_build_then_exact_resumable_case_shards",
        "sentinel_shard_id": "sentinel",
        "aggregation_order": "plan_order",
        "shards": shards.expected_specs(family),
    }


def header(family: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "producer_id": shards.PRODUCER_IDS[family],
        "family": (
            "conditional_process_v2"
            if family == "conditional"
            else "interventional_causal_mediation_v1"
        ),
        "scale": "qualification",
        "seed": 42,
        "metamorphism": "baseline",
        "sign_columns": None,
        "workers": 1,
        "qualification_claim": "none",
        "execution_contract": "public_test_contract",
        "required_cell_ids": [],
    }


def result(
    family: str, row: dict[str, object]
) -> dict[str, object]:
    shard_id = str(row["shard_id"])
    scientific_identity = row["scientific_identity"]
    if shard_id == "sentinel":
        value: dict[str, object] = {
            "sentinel_id": scientific_identity["identity_id"],
            "header": header(family),
            "qualification_boundary_guards": {"poison": "must_not_be_promoted"},
            "assumption_and_scope_blockers": {"poison": "must_not_be_promoted"},
        }
    elif row["payload_kind"] == "evidence":
        value = {"evidence_id": scientific_identity["identity_id"]}
        if family == "conditional":
            value["qualification_boundary_guards"] = {"guard": "blocked"}
            value["unsupported_intersections"] = {"intersection": "blocked"}
        else:
            value["assumption_and_scope_blockers"] = {"assumption": "blocked"}
    else:
        value = {"case_id": scientific_identity["case_id"]}
        if "fixture_role" in scientific_identity:
            value["fixture_role"] = scientific_identity["fixture_role"]
        if "alternative" in scientific_identity:
            value["result"] = {
                "targets": [
                    {"interval": {"alternative": scientific_identity["alternative"]}}
                ]
            }
    return {
        "schema_version": 1,
        "suite_id": shards.SHARD_SUITE_ID,
        "family": family,
        "producer_id": shards.PRODUCER_IDS[family],
        "shard_id": shard_id,
        "scale": "qualification",
        "seed": 42,
        "metamorphism": "baseline",
        "sign_columns": None,
        "workers": 1,
        "dependency_shard_ids": sorted(row["dependencies"]),
        "scientific_identity": scientific_identity,
        "payload": {"kind": row["payload_kind"], "value": value},
    }


class ConditionalCausalShardContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.shard_dir = self.root / "shards"
        self.shard_dir.mkdir()
        self.plan_path = self.root / "plan.json"
        self.executable_path = self.root / "producer.exe"
        self.executable_path.write_bytes(b"conditional-causal-test-producer")
        self.executable_sha256 = shards.sha256_file(self.executable_path)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_causal_public_interpretation_label_is_exact_and_runner_owned(self) -> None:
        exact = {
            "result": {
                "interpretation_label": verifier.CAUSAL_INTERPRETATION_LABEL,
            }
        }
        self.assertTrue(verifier.causal_interpretation_label_is_valid(exact))
        drifted = copy.deepcopy(exact)
        drifted["result"]["interpretation_label"] += (
            "; causality is not established by the calculation"
        )
        self.assertFalse(verifier.causal_interpretation_label_is_valid(drifted))

        runner_root = Path(__file__).resolve().parents[2] / "crates" / "qpls-runner" / "src"
        for source_name in ("multimod_causal_raw_v1.rs", "multimod_execution_v1.rs"):
            source = (runner_root / source_name).read_text(encoding="utf-8")
            with self.subTest(source=source_name):
                self.assertRegex(
                    source,
                    re.compile(
                        r"interpretation_label:\s*"
                        r"INTERVENTIONAL_MEDIATION_RESULT_INTERPRETATION_LABEL_V1\.into\(\)"
                    ),
                )
                self.assertNotRegex(
                    source,
                    re.compile(
                        r"interpretation_label:\s*"
                        r"qpls_estimation::INTERVENTIONAL_MEDIATION_INTERPRETATION_V1\.into\(\)"
                    ),
                )

    def seal(self, family: str, row: dict[str, object]) -> None:
        shard_id = str(row["shard_id"])
        temporary_result = self.root / f"{shard_id}.tmp.json"
        shards.atomic_json(temporary_result, result(family, row))
        shards.seal(
            types.SimpleNamespace(
                family=family,
                plan=self.plan_path,
                shard_dir=self.shard_dir,
                shard_id=shard_id,
                temporary_result=temporary_result,
                executable_sha256=self.executable_sha256,
                source_commit=SOURCE_COMMIT,
            )
        )

    def test_exact_frozen_inventory(self) -> None:
        conditional = shards.expected_specs("conditional")
        causal = shards.expected_specs("causal")
        self.assertEqual(len(conditional), 21)
        self.assertEqual(len(causal), 5)
        self.assertEqual(conditional[0]["shard_id"], "sentinel")
        self.assertEqual(causal[0]["shard_id"], "sentinel")
        self.assertEqual(conditional[1]["shard_id"], "qualification-guards")
        self.assertEqual(causal[1]["shard_id"], "assumption-scope-guards")
        self.assertEqual(
            [row["shard_id"] for row in conditional[-4:]],
            ["hoc", "grouped", "case-weighted", "frequency-weighted"],
        )

    def test_sealed_checkpoint_rejects_result_tampering(self) -> None:
        family = "conditional"
        rows = shards.expected_specs(family)
        shards.atomic_json(self.plan_path, plan(family))
        self.seal(family, rows[0])
        shards.verify_checkpoint(
            self.plan_path,
            self.shard_dir,
            "sentinel",
            self.executable_sha256,
            SOURCE_COMMIT,
        )
        altered = json.loads(
            (self.shard_dir / "sentinel.json").read_text(encoding="utf-8")
        )
        altered["seed"] = 43
        shards.atomic_json(self.shard_dir / "sentinel.json", altered)
        with self.assertRaises(shards.ContractError):
            shards.verify_checkpoint(
                self.plan_path,
                self.shard_dir,
                "sentinel",
                self.executable_sha256,
                SOURCE_COMMIT,
            )

    def test_child_receipt_binds_exact_sentinel_receipt(self) -> None:
        family = "causal"
        rows = shards.expected_specs(family)
        shards.atomic_json(self.plan_path, plan(family))
        self.seal(family, rows[0])
        self.seal(family, rows[1])
        dependency_receipt = json.loads(
            (self.shard_dir / "sentinel.receipt.json").read_text(encoding="utf-8")
        )
        dependency_receipt["result_sha256"] = "0" * 64
        shards.atomic_json(
            self.shard_dir / "sentinel.receipt.json", dependency_receipt
        )
        with self.assertRaises(shards.ContractError):
            shards.verify_checkpoint(
                self.plan_path,
                self.shard_dir,
                str(rows[1]["shard_id"]),
                self.executable_sha256,
                SOURCE_COMMIT,
            )

    def test_case_shard_rejects_swapped_alternative_and_role_payloads(self) -> None:
        family = "conditional"
        rows = shards.expected_specs(family)
        by_id = {str(row["shard_id"]): row for row in rows}
        shards.atomic_json(self.plan_path, plan(family))
        self.seal(family, by_id["sentinel"])
        for target_id, source_id in (
            ("multi-path-less", "multi-path-greater"),
            ("bca-null-two_sided", "bca-non-null-two_sided"),
        ):
            with self.subTest(target_id=target_id, source_id=source_id):
                target = result(family, by_id[target_id])
                target["payload"] = result(family, by_id[source_id])["payload"]
                temporary_result = self.root / f"{target_id}.swapped.tmp.json"
                shards.atomic_json(temporary_result, target)
                with self.assertRaises(shards.ContractError):
                    shards.seal(
                        types.SimpleNamespace(
                            family=family,
                            plan=self.plan_path,
                            shard_dir=self.shard_dir,
                            shard_id=target_id,
                            temporary_result=temporary_result,
                            executable_sha256=self.executable_sha256,
                            source_commit=SOURCE_COMMIT,
                        )
                    )

    def test_deterministic_aggregation_preserves_exact_case_order(self) -> None:
        for family, expected_case_count in (("conditional", 19), ("causal", 3)):
            with self.subTest(family=family):
                family_root = self.root / family
                family_root.mkdir()
                self.plan_path = family_root / "plan.json"
                self.shard_dir = family_root / "shards"
                self.shard_dir.mkdir()
                rows = shards.expected_specs(family)
                shards.atomic_json(self.plan_path, plan(family))
                for row in rows:
                    self.seal(family, row)
                output = family_root / "aggregate.json"
                shards.aggregate(
                    types.SimpleNamespace(
                        family=family,
                        plan=self.plan_path,
                        shard_dir=self.shard_dir,
                        executable_sha256=self.executable_sha256,
                        source_commit=SOURCE_COMMIT,
                        output=output,
                    )
                )
                aggregate = json.loads(output.read_text(encoding="utf-8"))
                expected_ids = [
                    row["shard_id"]
                    for row in rows
                    if row["payload_kind"] == "case"
                ]
                expected_case_ids = [
                    row["scientific_identity"]["case_id"]
                    for row in rows
                    if row["payload_kind"] == "case"
                ]
                self.assertEqual(
                    [case["case_id"] for case in aggregate["cases"]], expected_case_ids
                )
                self.assertEqual(len(aggregate["cases"]), expected_case_count)
                self.assertEqual(
                    [case["qualification_shard_id"] for case in aggregate["cases"]],
                    expected_ids,
                )
                if family == "conditional":
                    self.assertEqual(
                        aggregate["qualification_boundary_guards"], {"guard": "blocked"}
                    )
                    self.assertNotIn("poison", aggregate["qualification_boundary_guards"])
                else:
                    self.assertEqual(
                        aggregate["assumption_and_scope_blockers"],
                        {"assumption": "blocked"},
                    )
                    self.assertNotIn("poison", aggregate["assumption_and_scope_blockers"])
                self.assertEqual(
                    [
                        row["shard_id"]
                        for row in aggregate["shard_execution_receipt"]["shards"]
                    ],
                    [row["shard_id"] for row in rows],
                )
                self.assertTrue(
                    verifier.verify_shard_execution_receipt(
                        aggregate,
                        family,
                        required=True,
                        plan_path=self.plan_path,
                        shard_dir=self.shard_dir,
                        producer_executable=self.executable_path,
                        expected_source_commit=SOURCE_COMMIT,
                    )[0]
                )
                aggregate["shard_execution_receipt"]["shards"].reverse()
                self.assertFalse(
                    verifier.verify_shard_execution_receipt(
                        aggregate,
                        family,
                        required=True,
                        plan_path=self.plan_path,
                        shard_dir=self.shard_dir,
                        producer_executable=self.executable_path,
                        expected_source_commit=SOURCE_COMMIT,
                    )[0]
                )

    def test_portable_verifier_recomputes_every_receipt_binding(self) -> None:
        family = "causal"
        rows = shards.expected_specs(family)
        shards.atomic_json(self.plan_path, plan(family))
        for row in rows:
            self.seal(family, row)
        aggregate = shards.build_aggregate_report(
            family,
            self.plan_path,
            self.shard_dir,
            self.executable_sha256,
            SOURCE_COMMIT,
        )
        arguments = {
            "plan_path": self.plan_path,
            "shard_dir": self.shard_dir,
            "producer_executable": self.executable_path,
            "expected_source_commit": SOURCE_COMMIT,
        }
        self.assertTrue(
            verifier.verify_shard_execution_receipt(
                aggregate, family, required=True, **arguments
            )[0]
        )
        mutations = (
            ("plan_sha256", None),
            ("producer_executable_sha256", None),
            ("source_commit", None),
            ("receipt_sha256", 0),
            ("result_sha256", 0),
        )
        for field, row_index in mutations:
            with self.subTest(field=field):
                altered = copy.deepcopy(aggregate)
                if row_index is None:
                    altered["shard_execution_receipt"][field] = "0" * (
                        40 if field == "source_commit" else 64
                    )
                else:
                    altered["shard_execution_receipt"]["shards"][row_index][field] = (
                        "0" * 64
                    )
                self.assertFalse(
                    verifier.verify_shard_execution_receipt(
                        altered, family, required=True, **arguments
                    )[0]
                )

    def test_wrapper_freezes_caps_baseline_and_bounded_children(self) -> None:
        wrapper = Path(__file__).with_name(
            "run_conditional_causal_raw_qualification_v1.ps1"
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
            'function Get-CompletedChildRuntimeSeconds',
            '$Job.Process.ExitTime.ToUniversalTime()',
            '$completedRuntimeSeconds -ge $Job.TimeoutSeconds',
            'WaitForExit(10000)',
            '-Stage "cargo-build"',
            '-Stage "plan"',
            '-Stage "aggregate"',
            '-Stage "verifier"',
            '"--producer-executable", $binary',
            '"--expected-source-commit", $sourceCommit',
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


if __name__ == "__main__":
    unittest.main()
