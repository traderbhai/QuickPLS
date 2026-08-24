#!/usr/bin/env python3
"""Contract tests for resumable production MGA qualification cells."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import multimod_mga_shards_v1 as shards


EXECUTABLE_SHA256 = "b" * 64
SOURCE_COMMIT = "a" * 40


class MgaShardContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.plan_path = self.root / "plan.json"
        self.cell_dir = self.root / "cells"
        self.cell_dir.mkdir()
        self.plan = shards.expected_plan("qualification", 42)
        shards.atomic_json(self.plan_path, self.plan)
        self.environment_sha256 = self.plan["baseline_environment_sha256"]

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def cell_result(self, spec: dict[str, object]) -> dict[str, object]:
        cell_id = str(spec["cell_id"])
        bindings: list[dict[str, object]] = []
        payload: dict[str, object] = {"cell_id": cell_id}
        if spec["production_cache_required"]:
            binding = {
                "suite_id": shards.CACHE_CHECKPOINT_SUITE_ID,
                "orchestration_cell_id": cell_id,
                "subfit_id": cell_id,
                "source_commit": SOURCE_COMMIT,
                "executable_sha256": EXECUTABLE_SHA256,
                "qualification_plan_sha256": shards.sha256_file(self.plan_path),
                "environment_sha256": self.environment_sha256,
                "scale": "qualification",
                "seed": 42,
                "production_plan_sha256": "c" * 64,
                "cache_sha256": "d" * 64,
                "finalized_cache_sha256": "e" * 64,
                "loaded_completed_shards": 2,
                "completed_shards": 4,
                "planned_shards": 4,
            }
            bindings.append(binding)
            payload["external_cache_binding"] = binding
        if cell_id == "mga-frequency-physically-expanded":
            payload["dataset_rows"] = 60
        return {
            "schema_version": 1,
            "suite_id": shards.CELL_RESULT_SUITE_ID,
            "producer_suite_id": shards.PRODUCER_SUITE_ID,
            "cell_id": cell_id,
            "payload_kind": spec["payload_kind"],
            "scale": "qualification",
            "seed": 42,
            "metamorphism": "baseline",
            "workers": 1,
            "source_commit": SOURCE_COMMIT,
            "executable_sha256": EXECUTABLE_SHA256,
            "qualification_plan_sha256": shards.sha256_file(self.plan_path),
            "environment_sha256": self.environment_sha256,
            "permutation_samples": 5_000,
            "bootstrap_samples": 5_000,
            "cache_bindings": bindings,
            "payload": payload,
        }

    def write_all_cells(self) -> None:
        for spec in self.plan["cells"]:
            temporary_result = self.root / f"{spec['cell_id']}.tmp.json"
            shards.atomic_json(temporary_result, self.cell_result(spec))
            shards.seal_cell(
                self.plan_path,
                self.cell_dir,
                str(spec["cell_id"]),
                temporary_result,
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
                self.environment_sha256,
            )

    def test_qualification_plan_has_exact_scientific_and_boundary_inventory(self) -> None:
        plan, by_id = shards.validated_plan(self.plan_path)
        expected_ids = [
            "mga-general-2-groups",
            "mga-general-3-groups",
            "mga-general-5-groups",
            "mga-general-20-groups",
            "mga-profile-multiple_two_way",
            "mga-profile-bounded_three_way",
            "mga-profile-bounded_two_way_moderated_mediation",
            "mga-profile-multiple_nonnested_hoc",
            "mga-profile-case_weighted_pls",
            "mga-profile-reflective_plsc",
            "mga-general-parametric-3-groups",
            "mga-frequency-compact",
            "mga-frequency-physically-expanded",
            "mga-label-forward",
            "mga-label-reverse",
            "mga-boundaries",
        ]
        self.assertEqual([row["cell_id"] for row in plan["cells"]], expected_ids)
        self.assertEqual(set(by_id), set(expected_ids))
        self.assertEqual(sum(row["production_cache_required"] for row in plan["cells"]), 15)
        self.assertEqual(plan["permutation_samples"], 5_000)
        self.assertEqual(plan["bootstrap_samples"], 5_000)

    def test_plan_rejects_reduced_scientific_ledger(self) -> None:
        altered = json.loads(json.dumps(self.plan))
        altered["permutation_samples"] = 999
        shards.atomic_json(self.plan_path, altered)
        with self.assertRaises(shards.ContractError):
            shards.validated_plan(self.plan_path)

    def test_completed_cell_rejects_identity_and_cache_tampering(self) -> None:
        spec = self.plan["cells"][0]
        temporary_result = self.root / f"{spec['cell_id']}.tmp.json"
        value = self.cell_result(spec)
        shards.atomic_json(temporary_result, value)
        shards.seal_cell(
            self.plan_path,
            self.cell_dir,
            str(spec["cell_id"]),
            temporary_result,
            EXECUTABLE_SHA256,
            SOURCE_COMMIT,
            self.environment_sha256,
        )
        shards.validate_completed_cell(
            self.plan_path,
            self.cell_dir,
            str(spec["cell_id"]),
            EXECUTABLE_SHA256,
            SOURCE_COMMIT,
            self.environment_sha256,
        )
        path = shards.result_path(self.cell_dir, str(spec["cell_id"]))
        value["payload"]["scientific_value"] = 1  # type: ignore[index]
        shards.atomic_json(path, value)
        with self.assertRaises(shards.ContractError):
            shards.validate_completed_cell(
                self.plan_path,
                self.cell_dir,
                str(spec["cell_id"]),
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
                self.environment_sha256,
            )
        del value["payload"]["scientific_value"]  # type: ignore[index]
        value["cache_bindings"][0]["completed_shards"] = 3  # type: ignore[index]
        shards.atomic_json(path, value)
        with self.assertRaises(shards.ContractError):
            shards.validate_completed_cell(
                self.plan_path,
                self.cell_dir,
                str(spec["cell_id"]),
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
                self.environment_sha256,
            )

    def test_aggregation_requires_exact_complete_inventory_and_binds_result_hashes(self) -> None:
        self.write_all_cells()
        output = self.root / "raw.json"
        shards.aggregate(
            self.plan_path,
            self.cell_dir,
            EXECUTABLE_SHA256,
            SOURCE_COMMIT,
            self.environment_sha256,
            output,
        )
        report = json.loads(output.read_text(encoding="utf-8"))
        receipt = report["qualification_cell_aggregation"]
        self.assertEqual(receipt["cell_count"], 16)
        self.assertEqual(
            [row["cell_id"] for row in receipt["cell_receipts"]],
            [row["cell_id"] for row in self.plan["cells"]],
        )
        self.assertTrue(
            all(shards.is_sha256(row["result_sha256"]) for row in receipt["cell_receipts"])
        )
        (self.cell_dir / "unexpected.json").write_text("{}\n", encoding="utf-8")
        with self.assertRaises(shards.ContractError):
            shards.aggregate(
                self.plan_path,
                self.cell_dir,
                EXECUTABLE_SHA256,
                SOURCE_COMMIT,
                self.environment_sha256,
                output,
            )

    def test_wrapper_freezes_build_sentinel_parallelism_and_time_caps(self) -> None:
        wrapper = Path(__file__).with_name(
            "run_multimod_mga_qualification_v1.ps1"
        ).read_text(encoding="utf-8")
        for fragment in (
            '[ValidateRange(1, 4)]',
            '[ValidateRange(60, 1800)]',
            '[ValidateRange(600, 6600)]',
            '$sentinelTimeoutSeconds = 120',
            '$cleanupReserveSeconds = 120',
            'git status --porcelain=v1 --untracked-files=all',
            'QPLS_MULTIMOD_METAMORPHISM_V1',
            'QPLS_MULTIMOD_WORKERS_V1',
            'QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1',
            'QPLS_MULTIMOD_SIGN_COLUMNS_V1',
            '"build", "--release", "--quiet", "--locked"',
            '$Job.Process.Kill($true)',
            '-Stage "cargo-build"',
            '-Stage "plan"',
            '-Stage "root-sentinel"',
            '-Stage "aggregate"',
            '-Stage "comparator"',
            'diagnostic-only g2 compiler/plan sentinel before any scientific cell',
            'function Get-CellCacheCheckpointCount',
            'function Get-VerifiedCellCacheProgress',
            '"--cache-status"',
            'qpls.multimod.mga.verified-cache-progress.v1',
            'without completing another production shard; refusing a wasteful retry',
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, wrapper)
        self.assertLess(
            wrapper.index('-Stage "root-sentinel"'),
            wrapper.index("$completed = [System.Collections.Generic.HashSet[string]]"),
        )
        self.assertNotIn("cargo run", wrapper)

    def test_rust_producer_uses_transactional_production_checkpoint_callback(self) -> None:
        producer = (
            Path(__file__).parents[2]
            / "crates/qpls-runner/examples/multimod_mga_qualification_v1.rs"
        ).read_text(encoding="utf-8")
        for fragment in (
            "run_compiled_raw_mga_resumable_with_checkpoint_v1(",
            '"--plan, --sentinel, and --cell are mutually exclusive"',
            '"qpls.multimod.mga.root-compiler-sentinel.v1"',
            '"qpls.multimod.mga.verified-cache-progress.v1"',
            '"scientific_result_published": false',
            '"deterministic_rebuild_preflight"',
            'prepare_cell_execution_plan("mga-general-20-groups", args.seed)',
            'MAX_GROUP_PLAN_SHARDS',
            "prepare_fixture_cell_authority(",
            "write_cache_status(&args, cell_id)",
            "let configured_metamorphism = env::var_os(metamorphic::METAMORPHISM_ENV_V1);",
            'value.to_str() != Some("baseline")',
            'value.to_str() != Some("1")',
            'env::var_os("QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1").is_some()',
            "checkpoint-{:06}-{:06}-{}.json",
            "cache_prefix_sha256",
            "file.sync_all()?",
            "fs::rename(&temporary, &destination)?",
            "const PERMUTATIONS: u32 = 5_000;",
            "const BOOTSTRAPS: u32 = 5_000;",
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, producer)

        support = (
            Path(__file__).parents[2]
            / "crates/qpls-runner/examples/support_multimod_qualification/mod.rs"
        ).read_text(encoding="utf-8")
        self.assertIn("qpls.multimod.qualification-dataset-id.v1", support)
        self.assertIn("dataset.id = Uuid::from_bytes(uuid_bytes);", support)

    def test_multiple_hoc_mga_uses_only_the_additive_base_projection(self) -> None:
        runner = (
            Path(__file__).parents[2]
            / "crates/qpls-runner/src/multimod_execution_v1.rs"
        ).read_text(encoding="utf-8")
        start = runner.index("fn projected_hoc_mga_authority_v1(")
        end = runner.index("fn run_hoc_mga_stage_v1", start)
        authority = runner[start:end]
        self.assertIn(
            "compile_pls_higher_order_lower_order_projection_multimod_v2",
            authority,
        )
        self.assertNotIn("project_general_sem_pls_stage_one_recipe_v1", authority)
        self.assertIn("if &base_stage.plan != plan.base_plan()", authority)

    def test_bounded_moderated_mediation_fixture_selects_exact_indirect_path(self) -> None:
        producer = (
            Path(__file__).parents[2]
            / "crates/qpls-runner/examples/multimod_mga_qualification_v1.rs"
        ).read_text(encoding="utf-8")
        start = producer.index("ProfileFixture::ModeratedMediation => {")
        end = producer.index("ProfileFixture::MultipleHoc => {", start)
        branch = producer[start:end]
        for fragment in (
            'relation_id(&model, "construct:x", "construct:m")?',
            'relation_id(&model, "construct:m", "construct:y")?',
            "GeneralSemEffectEstimandV1::SpecificPath",
            'estimand_id: "estimand:x_via_m_to_y".into()',
            "ordered_relation_ids: vec![first_stage_relation, second_stage_relation]",
            "GeneralSemInferenceV1::CaseBootstrap",
            "interval: GeneralSemBootstrapIntervalV1::Percentile",
            "tail: GeneralSemInferenceTailV1::TwoSided",
            "recipe.settings.bootstrap_samples = BOOTSTRAPS;",
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, branch)

    def test_gate_binds_checkpoint_authority_and_wrapper_cap(self) -> None:
        bindings = json.loads(
            (Path(__file__).with_name("multimod_gate_bindings_v1.json")).read_text(
                encoding="utf-8"
            )
        )
        gate = next(row for row in bindings["gates"] if row["gate_id"] == "mga.group_matrix")
        self.assertEqual(
            [row["step_id"] for row in gate["steps"][:2]],
            ["mga_shard_contracts", "mga_production_science"],
        )
        science = next(row for row in gate["steps"] if row["step_id"] == "mga_production_science")
        contracts = next(row for row in gate["steps"] if row["step_id"] == "mga_shard_contracts")
        self.assertEqual(science["maximum_seconds"], 6_600)
        self.assertEqual(science["expected_outputs"], ["{gate_output}/mga-production-science.json"])
        self.assertFalse(contracts["uses_cargo"])
        for artifact in (
            "crates/qpls-runner/src/multimod_mga_execution_cache_v1.rs",
            "validation/multimod/multimod_mga_shards_v1.py",
            "validation/multimod/test_multimod_mga_shards_v1.py",
        ):
            self.assertIn(artifact, gate["input_artifacts"])


if __name__ == "__main__":
    unittest.main()
