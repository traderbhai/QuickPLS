#!/usr/bin/env python3
"""Focused contract tests for the bounded maximum-profile topology."""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import types
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import multimod_heterogeneity_shards_v2 as heterogeneity
import multimod_mga_shards_v1 as mga
import multimod_performance_topology_v2 as topology
import verify_multimod_performance_profiles_v2 as verifier


SOURCE_COMMIT = "a" * 40


def heterogeneity_plan() -> dict[str, object]:
    return {
        "schema_version": 1,
        "suite_id": heterogeneity.PLAN_SUITE_ID,
        "producer_suite_id": heterogeneity.PRODUCER_SUITE_ID,
        "scale": "qualification",
        "campaign_seed": 42,
        "metamorphism": "baseline",
        "sign_columns": None,
        "workers": 1,
        "fixture_observations": 400,
        "execution_contract": "one_cargo_build_then_dependency_aware_non_cargo_shards",
        "sentinel_shard_id": "sentinel",
        "aggregation_order": "plan_order",
        "shards": heterogeneity.expected_shard_specs("qualification"),
    }


class PerformanceTopologyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.mga_plan = self.root / "mga" / "cell-plan.json"
        self.heterogeneity_plan = self.root / "heterogeneity" / "shard-plan.json"
        mga.atomic_json(self.mga_plan, mga.expected_plan("qualification", 42))
        heterogeneity.atomic_json(self.heterogeneity_plan, heterogeneity_plan())
        self.cargo_lock = self.root / "Cargo.lock"
        self.cargo_lock.write_text("lock-v1\n", encoding="utf-8")
        self.executables: dict[str, Path] = {}
        for executable_id in ("cargo", "mga", "heterogeneity", "maximum"):
            path = self.root / f"{executable_id}.exe"
            path.write_bytes(f"executable:{executable_id}".encode())
            self.executables[executable_id] = path
        self.build_attempt = self.root / "build" / "build-attempt.json"
        topology.atomic_json(
            self.build_attempt,
            {
                "schema_version": topology.SCHEMA_VERSION,
                "suite_id": topology.BUILD_ATTEMPT_SUITE_ID,
                "status": "completed",
                "attempt_id": "attempt-1",
                "attempt_count": 1,
                "source_commit": SOURCE_COMMIT,
                "cargo_executable_sha256": topology.sha256_file(
                    self.executables["cargo"]
                ),
                "cargo_lock_sha256": topology.sha256_file(self.cargo_lock),
                "command_identity": topology.stage_specs(42)[0][
                    "command_identities"
                ][0],
                "output_executables": {
                    executable_id: {
                        "file_name": self.executables[executable_id].name,
                        "sha256": topology.sha256_file(
                            self.executables[executable_id]
                        ),
                    }
                    for executable_id in ("mga", "heterogeneity", "maximum")
                },
                "measurement": {
                    "wall_time_milliseconds": 1000,
                    "peak_working_set_bytes": 1024,
                    "sampled_concurrent_peak_working_set_bytes": 512,
                    "per_process_peak_working_set_bytes": [
                        {
                            "process_identity": "release-build",
                            "peak_working_set_bytes": 1024,
                        }
                    ],
                    "working_set_sample_count": 2,
                    "process_count": 1,
                    "process_exit_codes": [0],
                },
            },
        )
        arguments = types.SimpleNamespace(
            seed=42,
            source_commit=SOURCE_COMMIT,
            mga_plan=self.mga_plan,
            heterogeneity_plan=self.heterogeneity_plan,
            cargo_lock=self.cargo_lock,
            build_attempt=self.build_attempt,
            cargo_executable=self.executables["cargo"],
            mga_executable=self.executables["mga"],
            heterogeneity_executable=self.executables["heterogeneity"],
            maximum_executable=self.executables["maximum"],
        )
        self.plan_path = self.root / "performance-topology-plan.json"
        topology.atomic_json(self.plan_path, topology.build_plan(arguments))
        self.plan, _ = topology.validated_plan(self.plan_path)
        for stage in self.plan["stages"]:
            for artifact in stage["artifacts"]:
                path = self.root / artifact["relative_path"]
                path.parent.mkdir(parents=True, exist_ok=True)
                if not path.exists():
                    path.write_text(
                        f"{artifact['role']}:{artifact['relative_path']}\n",
                        encoding="utf-8",
                    )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def measurement(self, stage: dict[str, object]) -> dict[str, object]:
        executable_id = str(stage["executable_id"])
        process_count = int(stage["process_count"])
        return {
            "schema_version": topology.SCHEMA_VERSION,
            "suite_id": topology.MEASUREMENT_SUITE_ID,
            "stage_id": stage["stage_id"],
            "topology_plan_sha256": topology.sha256_file(self.plan_path),
            "source_commit": SOURCE_COMMIT,
            "executable_sha256": self.plan["executables"][executable_id]["sha256"],
            "command_identities": stage["command_identities"],
            "wall_time_milliseconds": 1000,
            "peak_working_set_bytes": 1024 * process_count,
            "sampled_concurrent_peak_working_set_bytes": 512 * process_count,
            "per_process_peak_working_set_bytes": [
                {
                    "process_identity": f"process-{index}",
                    "peak_working_set_bytes": 1024,
                }
                for index in range(process_count)
            ],
            "working_set_sample_count": 2,
            "process_count": process_count,
            "process_exit_codes": [0] * process_count,
            "atomic_timeout_seconds": stage["atomic_timeout_seconds"],
        }

    def seal_all(self) -> None:
        for stage in self.plan["stages"]:
            measurement_path = self.root / f".{stage['stage_id']}.measurement.tmp.json"
            topology.atomic_json(measurement_path, self.measurement(stage))
            topology.seal_checkpoint(
                types.SimpleNamespace(
                    plan=self.plan_path,
                    work_root=self.root,
                    receipt_root=self.root,
                    stage_id=stage["stage_id"],
                    temporary_measurement=measurement_path,
                )
            )

    def test_plan_freezes_only_exact_maximum_cells_and_counts(self) -> None:
        self.assertEqual(self.plan["mga_permutation_samples"], 5000)
        self.assertEqual(self.plan["mga_bootstrap_samples"], 5000)
        self.assertEqual(self.plan["heterogeneity_bootstrap_samples"], 500)
        self.assertEqual(self.plan["internal_timeout_seconds"], 6480)
        stages = {row["stage_id"]: row for row in self.plan["stages"]}
        self.assertEqual(
            list(stages),
            [
                "release-build",
                "mga-general-20-groups",
                "heterogeneity-sentinel",
                "heterogeneity-p23-discovery",
                "heterogeneity-p23-bootstrap-pair",
                "conditional-sidecar-resume",
            ],
        )
        bootstrap = stages["heterogeneity-p23-bootstrap-pair"]
        self.assertEqual(bootstrap["process_count"], 2)
        self.assertEqual(bootstrap["parallel_processes"], 2)
        self.assertEqual(
            bootstrap["dependencies"],
            [
                "release-build",
                "heterogeneity-sentinel",
                "heterogeneity-p23-discovery",
            ],
        )
        commands = json.dumps(bootstrap["command_identities"])
        self.assertIn("bootstrap-fimix-p23", commands)
        self.assertIn("bootstrap-pos-destination-p23", commands)

    def test_receipts_bind_artifacts_dependencies_and_aggregate_topology(self) -> None:
        self.seal_all()
        aggregate = topology.build_aggregate(self.plan_path, self.root, self.root)
        workloads = {row["workload_id"]: row for row in aggregate["workloads"]}
        heterogeneity_workload = workloads["heterogeneity_locked_p23"]
        self.assertEqual(heterogeneity_workload["atomic_process_count"], 4)
        self.assertEqual(heterogeneity_workload["wall_time_milliseconds"], 3000)
        self.assertEqual(heterogeneity_workload["peak_working_set_bytes"], 2048)
        self.assertGreater(heterogeneity_workload["output_size_bytes"], 0)
        bootstrap_receipt = topology.read_object(
            topology.checkpoint_directory(
                self.root, "heterogeneity-p23-bootstrap-pair"
            )
            / "receipt.json"
        )
        self.assertEqual(
            [row["stage_id"] for row in bootstrap_receipt["dependency_receipts"]],
            [
                "release-build",
                "heterogeneity-sentinel",
                "heterogeneity-p23-discovery",
            ],
        )
        portable_root = self.root.parent / f"{self.root.name}-portable-copy"
        shutil.copytree(self.root, portable_root)
        try:
            measurement = topology.verify_checkpoint(
                portable_root / "performance-topology-plan.json",
                portable_root,
                portable_root,
                "heterogeneity-p23-bootstrap-pair",
            )
            self.assertEqual(measurement["process_count"], 2)
        finally:
            shutil.rmtree(portable_root)

    def test_artifact_tamper_and_wrong_dependency_plan_fail_closed(self) -> None:
        self.seal_all()
        target = self.root / "maximum" / "result.json"
        target.write_text("tampered\n", encoding="utf-8")
        with self.assertRaisesRegex(topology.ContractError, "stale or altered"):
            topology.verify_checkpoint(
                self.plan_path, self.root, self.root, "conditional-sidecar-resume"
            )

        invalid = heterogeneity_plan()
        for row in invalid["shards"]:  # type: ignore[index]
            if row["shard_id"] == "bootstrap-fimix-p23":
                row["dependencies"] = ["sentinel"]
        invalid_path = self.root / "invalid-heterogeneity-plan.json"
        heterogeneity.atomic_json(invalid_path, invalid)
        arguments = types.SimpleNamespace(
            seed=42,
            source_commit=SOURCE_COMMIT,
            mga_plan=self.mga_plan,
            heterogeneity_plan=invalid_path,
            cargo_lock=self.cargo_lock,
            build_attempt=self.build_attempt,
            cargo_executable=self.executables["cargo"],
            mga_executable=self.executables["mga"],
            heterogeneity_executable=self.executables["heterogeneity"],
            maximum_executable=self.executables["maximum"],
        )
        with self.assertRaises(heterogeneity.ContractError):
            topology.build_plan(arguments)

    def test_conservative_peak_and_release_build_attempt_fail_closed(self) -> None:
        _, stages = topology.validated_plan(self.plan_path)
        bootstrap = self.measurement(stages["heterogeneity-p23-bootstrap-pair"])
        bootstrap["peak_working_set_bytes"] = bootstrap[
            "sampled_concurrent_peak_working_set_bytes"
        ]
        with self.assertRaisesRegex(topology.ContractError, "identity or bounds"):
            topology.validate_measurement(
                bootstrap,
                self.plan,
                stages["heterogeneity-p23-bootstrap-pair"],
            )

        attempt = topology.read_object(self.build_attempt)
        attempt["attempt_count"] = 2
        topology.atomic_json(self.build_attempt, attempt)
        with self.assertRaisesRegex(topology.ContractError, "release-build attempt"):
            topology.measured_artifacts(
                self.root,
                stages["release-build"],
                self.plan,
            )

    def test_wrapper_and_gate_bind_the_bounded_direct_execution_contract(self) -> None:
        here = Path(__file__).resolve().parent
        wrapper = (here / "run_multimod_performance_profiles_v2.ps1").read_text(
            encoding="utf-8"
        )
        self.assertNotIn('"run", "--release"', wrapper)
        self.assertEqual(wrapper.count('"cargo", "build", "--release"'), 1)
        for example in (
            "multimod_mga_qualification_v1",
            "multimod_heterogeneity_qualification_v2",
            "multimod_maximum_profiles_performance_v1",
        ):
            self.assertIn(example, wrapper)
        self.assertIn("Invoke-HeterogeneityBootstrapPair", wrapper)
        self.assertIn('suite_id = "qpls.v256.multimod.performance-release-build-attempt.v2"', wrapper)
        self.assertIn('if (-not (Test-PerformanceCheckpoint "release-build"))', wrapper)
        self.assertIn("$executionDeadlineSeconds = $InternalTimeoutSeconds - $publicationReserveSeconds", wrapper)
        self.assertIn("Assert-PublicationDeadline", wrapper)
        self.assertIn("Remove-Item -LiteralPath $outputPath -Force", wrapper)
        self.assertIn("Update-ChildPeakWorkingSets", wrapper)
        self.assertIn("result_receipt_transaction_incomplete", wrapper)
        self.assertIn("qpls.v256.multimod.maximum-profile-performance.v2", wrapper)
        self.assertIn("[ValidateRange(1, 1800)]", wrapper)
        self.assertIn("[ValidateRange(1, 6480)]", wrapper)
        bindings = json.loads(
            (here / "multimod_gate_bindings_v1.json").read_text(encoding="utf-8")
        )
        gate = next(
            row
            for row in bindings["gates"]
            if row["gate_id"] == "performance.maximum_profiles"
        )
        production_step = next(
            row
            for row in gate["steps"]
            if row["step_id"] == "maximum_profile_production_performance"
        )
        self.assertEqual(production_step["maximum_seconds"], 6600)
        self.assertIn(
            "{campaign_root}/execution-cache/performance-maximum-profiles-v2",
            production_step["arguments"],
        )
        self.assertIn(
            "validation/multimod/multimod_performance_topology_v2.py",
            gate["input_artifacts"],
        )
        for artifact in (
            "validation/multimod/run_multimod_performance_profiles_v2.ps1",
            "validation/multimod/verify_multimod_performance_profiles_v2.py",
            "validation/multimod/test_multimod_performance_topology_v2.py",
            "validation/multimod/multimod_maximum_profile_performance_v2.schema.json",
            "validation/multimod/multimod_performance_output_verification_v2.schema.json",
        ):
            self.assertIn(artifact, gate["input_artifacts"])

    def test_public_v2_reports_are_explicit_and_v1_files_remain_v1(self) -> None:
        here = Path(__file__).resolve().parent
        wrapper_v2 = (here / "run_multimod_performance_profiles_v2.ps1").read_text(
            encoding="utf-8"
        )
        verifier_v2 = (
            here / "verify_multimod_performance_profiles_v2.py"
        ).read_text(encoding="utf-8")
        wrapper_v1 = (here / "run_multimod_performance_profiles_v1.ps1").read_text(
            encoding="utf-8"
        )
        verifier_v1 = (
            here / "verify_multimod_performance_profiles_v1.py"
        ).read_text(encoding="utf-8")
        maximum_schema = json.loads(
            (here / "multimod_maximum_profile_performance_v2.schema.json").read_text(
                encoding="utf-8"
            )
        )
        verification_schema = json.loads(
            (
                here / "multimod_performance_output_verification_v2.schema.json"
            ).read_text(encoding="utf-8")
        )
        self.assertIn("qpls.v256.multimod.maximum-profile-performance.v2", wrapper_v2)
        self.assertIn("qpls.v256.multimod.performance-output-verification.v2", verifier_v2)
        self.assertIn("qpls.v256.multimod.maximum-profile-performance.v1", wrapper_v1)
        self.assertIn("qpls.v256.multimod.performance-output-verification.v1", verifier_v1)
        self.assertEqual(maximum_schema["properties"]["schema_version"]["const"], 2)
        self.assertEqual(verification_schema["properties"]["schema_version"]["const"], 2)

    def test_verifier_accepts_only_exact_cell_and_dependency_envelopes(self) -> None:
        pairs = [
            {
                "left_group_id": f"g{left:02}",
                "right_group_id": f"g{right:02}",
                "procedure": "pairwise_permutation_two_sided",
            }
            for left in range(20)
            for right in range(left + 1, 20)
        ]
        mga_document = {
            "suite_id": "qpls.multimod.mga.qualification-cell-result.v1",
            "cell_id": "mga-general-20-groups",
            "payload_kind": "group_matrix_cell",
            "scale": "qualification",
            "seed": 42,
            "permutation_samples": 5000,
            "bootstrap_samples": 5000,
            "metamorphism": "baseline",
            "workers": 1,
            "qualification_plan_sha256": "1" * 64,
            "executable_sha256": "2" * 64,
            "cache_bindings": [{}],
            "payload": {
                "group_count": 20,
                "expected_pair_count": 190,
                "heavy_run_confirmed": True,
                "analysis": {"pairwise": pairs, "omnibus": [{}]},
                "execution_plan": {
                    "retry_policy": "none",
                    "shards": [{}] * 191,
                    "plan_sha256": "3" * 64,
                    "dataset_fingerprint": "4" * 64,
                    "analysis_identity_sha256": "5" * 64,
                },
            },
        }
        self.assertEqual(verifier.verify_mga(mga_document)["pair_count"], 190)
        broad = dict(mga_document)
        broad["suite_id"] = "qpls.multimod.mga.production-qualification.v1"
        with self.assertRaisesRegex(ValueError, "exact cell envelope"):
            verifier.verify_mga(broad)

        discovery_identity = "d" * 64

        def envelope(
            shard_id: str, kind: str, dependencies: list[str], value: dict[str, object]
        ) -> dict[str, object]:
            return {
                "suite_id": "qpls.multimod.heterogeneity.qualification-shard.v1",
                "shard_id": shard_id,
                "scale": "qualification",
                "campaign_seed": 42,
                "metamorphism": "baseline",
                "workers": 1,
                "dependency_shard_ids": dependencies,
                "payload": {"kind": kind, "value": value},
            }

        def bootstrap_cell(cell_id: str, algorithm: str) -> dict[str, object]:
            return {
                "cell_id": cell_id,
                "config": {
                    "profile": "p23_all_current",
                    "phase": {
                        "kind": "inference",
                        "lock": {
                            "selected_algorithm": algorithm,
                            "selected_k": 2,
                            "discovery_result_identity_sha256": discovery_identity,
                        },
                    },
                    "bootstrap": {"resamples": 500},
                },
                "evidence": {"bootstrap": [{}]},
                "analysis": {"bootstrap_ledger": {"requested": 500}},
                "compiler_receipt": {
                    "dataset_fingerprint": "a" * 64,
                    "recipe_analytical_sha256": "b" * 64,
                    "model_scientific_sha256": "c" * 64,
                },
            }

        documents = {
            "sentinel": envelope(
                "sentinel",
                "sentinel",
                [],
                {
                    "claim": "fail_fast_pipeline_sentinel_not_scientific_acceptance_evidence"
                },
            ),
            "pos-destination-p23-discovery": envelope(
                "pos-destination-p23-discovery",
                "pos_destination_p23_discovery",
                ["sentinel"],
                {"analysis": {"discovery_result_identity_sha256": discovery_identity}},
            ),
            "bootstrap-fimix-p23": envelope(
                "bootstrap-fimix-p23",
                "bootstrap",
                ["pos-destination-p23-discovery", "sentinel"],
                bootstrap_cell("fimix-p23-fixed-k-bootstrap", "fimix_pls_v2"),
            ),
            "bootstrap-pos-destination-p23": envelope(
                "bootstrap-pos-destination-p23",
                "bootstrap",
                ["pos-destination-p23-discovery", "sentinel"],
                bootstrap_cell(
                    "pos-destination-p23-fixed-k-bootstrap",
                    "pls_pos_destination_scored_interactions_v2",
                ),
            ),
        }
        verified = verifier.verify_heterogeneity(documents)
        self.assertEqual(len(verified["locked_maximum_profile_cells"]), 2)
        documents["bootstrap-fimix-p23"]["dependency_shard_ids"] = ["sentinel"]
        with self.assertRaisesRegex(ValueError, "dependency envelope differs"):
            verifier.verify_heterogeneity(documents)


if __name__ == "__main__":
    unittest.main()
