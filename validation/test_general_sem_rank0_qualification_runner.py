#!/usr/bin/env python3
"""Focused integrity tests for the resumable Rank 0 qualification runner."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import general_sem_rank0_qualification_runner as runner  # noqa: E402


class GeneralSemRank0QualificationRunnerTests(unittest.TestCase):
    def temporary_output(self):
        return tempfile.TemporaryDirectory(prefix="qpls-rank0-runner-", dir=ROOT)

    def test_planner_is_deterministic_pairwise_complete_and_exactly_sharded(
        self,
    ) -> None:
        first = runner.build_plan(
            qualification_trials=10,
            shard_size=4,
            included_suites=(
                "pairwise",
                "failure_classification",
                "independent_oracle_comparison",
                "current_product_comparison",
            ),
        )
        replay = runner.build_plan(
            qualification_trials=10,
            shard_size=4,
            included_suites=(
                "pairwise",
                "failure_classification",
                "independent_oracle_comparison",
                "current_product_comparison",
            ),
        )
        self.assertEqual(first, replay)
        runner.validate_plan(first)
        self.assertEqual(
            first["execution_policy"]["default_concurrency"],
            runner.DEFAULT_CONCURRENCY,
        )
        self.assertEqual(runner.DEFAULT_CONCURRENCY, 4)
        self.assertTrue(
            {
                "pairwise",
                "failure_classification",
                "independent_oracle_comparison",
                "current_product_comparison",
            }
            <= {scenario["suite"] for scenario in first["scenarios"]}
        )
        axes = {
            "topology": ("parallel", "serial", "mixed"),
            "mode": ("a", "ab"),
            "distribution": ("gaussian", "heavy"),
            "missingness": ("complete", "mcar"),
        }
        cases = runner.greedy_pairwise_cases(axes)
        self.assertFalse(runner.uncovered_pairs(axes, cases))
        for scenario in first["scenarios"]:
            ranges = sorted(
                (
                    shard["trial_start_inclusive"],
                    shard["trial_stop_exclusive"],
                )
                for shard in first["shards"]
                if shard["scenario_id"] == scenario["scenario_id"]
            )
            cursor = 0
            for start, stop in ranges:
                self.assertEqual(start, cursor)
                cursor = stop
            self.assertEqual(cursor, scenario["trial_count"])
        self.assertTrue(
            all(
                0 <= shard["first_trial_seed"] <= runner.MAX_SAFE_GENERAL_SEM_SEED
                and 0 <= shard["last_trial_seed"] <= runner.MAX_SAFE_GENERAL_SEM_SEED
                for shard in first["shards"]
            )
        )

    def test_plan_tamper_is_rejected(self) -> None:
        plan = runner.build_plan(
            qualification_trials=4,
            shard_size=2,
            included_suites=("failure_classification",),
        )
        tampered = json.loads(json.dumps(plan))
        tampered["shards"][0]["trial_stop_exclusive"] += 1
        with self.assertRaises(runner.PlanValidationError):
            runner.validate_plan(tampered)

    def test_full_plan_validator_rejects_a_valid_focused_plan(self) -> None:
        focused = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("current_product_comparison",),
        )
        with mock.patch.object(runner, "build_plan", return_value=focused):
            runner.validate_frozen_full_plan(focused)
            different = json.loads(json.dumps(focused))
            different["qualification_trials"] = 2
            different["plan_sha256"] = runner.canonical_sha256(
                {key: value for key, value in different.items() if key != "plan_sha256"}
            )
            with self.assertRaises(runner.PlanValidationError):
                runner.validate_frozen_full_plan(different)

    def test_resource_guard_stops_at_either_drive_boundary(self) -> None:
        def healthy(path: str) -> float:
            return 40.0 if path.startswith("C") else 50.0

        self.assertEqual(runner.enforce_resource_guard(healthy), {"C": 40.0, "D": 50.0})
        with self.assertRaises(runner.ResourceGuardError):
            runner.enforce_resource_guard(
                lambda path: 21.99 if path.startswith("C") else 50.0
            )
        with self.assertRaises(runner.ResourceGuardError):
            runner.enforce_resource_guard(
                lambda path: 40.0 if path.startswith("C") else 27.99
            )

    def test_cross_oracle_failure_messages_map_to_one_scientific_code(self) -> None:
        self.assertEqual(
            runner._failure_reason_code("PLS-PM did not converge in 3000 iterations"),  # noqa: SLF001
            "estimation_nonconvergence",
        )
        self.assertEqual(
            runner._failure_reason_code("cSEM PLS-PM did not converge"),  # noqa: SLF001
            "estimation_nonconvergence",
        )

    def test_completed_shard_resumes_without_overwrite(self) -> None:
        plan = runner.build_plan(
            qualification_trials=4,
            shard_size=4,
            included_suites=("failure_classification",),
        )
        with self.temporary_output() as raw:
            output = Path(raw)
            runner.publish_plan(plan, output)
            shard_id = plan["shards"][0]["shard_id"]
            first = runner.execute_shard(plan, shard_id, output)
            path = Path(first["path"])
            before = path.read_bytes()
            modified = path.stat().st_mtime_ns
            replay = runner.execute_shard(plan, shard_id, output)
            self.assertEqual(replay["status"], "accepted_existing")
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(path.stat().st_mtime_ns, modified)

    def test_dead_claim_is_quarantined_and_resumed(self) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("failure_classification",),
        )
        with self.temporary_output() as raw:
            output = Path(raw)
            runner.publish_plan(plan, output)
            shard_id = plan["shards"][0]["shard_id"]
            claim = runner.claim_path(output, shard_id)
            claim.parent.mkdir(parents=True)
            claim.write_bytes(
                runner.canonical_bytes(
                    {
                        "schema_version": 1,
                        "kind": "general_sem_rank0_shard_claim_v1",
                        "plan_sha256": plan["plan_sha256"],
                        "shard_id": shard_id,
                        "host": "dead-host",
                        "pid": 999_999_999,
                        "created_unix_ns": 1,
                        "token": "dead",
                    }
                )
            )
            result = runner.execute_shard(
                plan, shard_id, output, stale_claim_seconds=0.0
            )
            self.assertEqual(result["status"], "published")
            self.assertTrue(list((claim.parent / "abandoned").glob("*.json")))

    def test_aggregation_is_completion_order_independent_and_missing_fails_closed(
        self,
    ) -> None:
        plan = runner.build_plan(
            qualification_trials=4,
            shard_size=2,
            included_suites=("failure_classification",),
        )
        with self.temporary_output() as raw:
            output = Path(raw)
            runner.publish_plan(plan, output)
            incomplete = runner.aggregate_plan(plan, output)
            self.assertEqual(incomplete["status"], "incomplete")
            self.assertFalse(incomplete["passed"])
            for shard in reversed(plan["shards"]):
                runner.execute_shard(plan, shard["shard_id"], output)
            first = runner.aggregate_plan(plan, output)
            second = runner.aggregate_plan(plan, output)
            self.assertEqual(first, second)
            first_path = runner.publish_aggregate(first, plan, output)
            second_path = runner.publish_aggregate(second, plan, output)
            self.assertEqual(first_path, second_path)

    def test_tampered_accepted_shard_is_never_overwritten_or_aggregated(self) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("failure_classification",),
        )
        with self.temporary_output() as raw:
            output = Path(raw)
            runner.publish_plan(plan, output)
            shard = plan["shards"][0]
            runner.execute_shard(plan, shard["shard_id"], output)
            path = runner.shard_path(output, shard["shard_id"])
            tampered = json.loads(path.read_text(encoding="utf-8"))
            tampered["metric_counts"]["failure_classification_rate"]["event_count"] = 0
            path.write_text(json.dumps(tampered), encoding="utf-8")
            with self.assertRaises(runner.ArtifactTamperError):
                runner.aggregate_plan(plan, output)
            with self.assertRaises(runner.ArtifactTamperError):
                runner.execute_shard(plan, shard["shard_id"], output)

    def test_product_request_binds_both_oracles_seed_workers_and_outputs(self) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("current_product_comparison",),
        )
        point_shard = next(
            shard
            for shard in plan["shards"]
            if runner._scenario_map(plan)[shard["scenario_id"]]["cell_key"]  # noqa: SLF001
            == "mediation_point"
        )
        request = runner.build_product_request(plan, point_shard["shard_id"])
        self.assertEqual(request["required_worker_axes"], ["not_applicable"])
        self.assertIsNone(request["bootstrap_seed"])
        contract = plan["current_product_comparison_contract"]
        self.assertEqual(len(contract["required_independent_oracles"]), 2)
        self.assertIn("outer_weights_and_loadings", contract["required_bindings"])
        self.assertIn("analytical_method_version", contract["required_bindings"])
        self.assertIn("bootstrap_summaries", contract["required_bindings"])
        self.assertIn("ordered_failure_ledger", contract["required_bindings"])
        self.assertEqual(request["kind"], runner.PRODUCT_REQUEST_KIND)
        self.assertEqual(request["method_version"], "pls_mediation_v1")
        self.assertEqual(request["analytical_method_version"], "pls_mediation_v1")
        self.assertEqual(
            request["product_input"]["scenario_id"], request["scenario_id"]
        )
        self.assertIsNone(request["product_input"]["bootstrap"])
        self.assertTrue(request["required_production_result"]["raw_typed_result"])
        self.assertEqual(
            request["cargo_execution"]["bundle_kind"], runner.PRODUCT_BUNDLE_KIND
        )
        bootstrap_shard = next(
            shard
            for shard in plan["shards"]
            if runner._scenario_map(plan)[shard["scenario_id"]]["cell_key"]  # noqa: SLF001
            == "mediation_bootstrap"
        )
        bootstrap_request = runner.build_product_request(
            plan, bootstrap_shard["shard_id"]
        )
        self.assertEqual(
            bootstrap_request["required_worker_axes"], ["1", "2", "4", "max"]
        )

    def test_plan_explicitly_covers_metamorphic_seed_and_worker_axes(self) -> None:
        plan = runner.build_plan(
            qualification_trials=2,
            shard_size=1,
            included_suites=("metamorphic_invariance", "seed_replay"),
        )
        metamorphic = [
            scenario
            for scenario in plan["scenarios"]
            if scenario["suite"] == "metamorphic_invariance"
        ]
        self.assertEqual(len(metamorphic), len(runner.CELL_CONTRACTS) * 5)
        self.assertEqual(
            {scenario["parameters"]["metamorphism"] for scenario in metamorphic},
            set(runner.METAMORPHISMS),
        )
        self.assertTrue(
            all(scenario["workload"]["resamples"] == 0 for scenario in metamorphic)
        )
        seed_scenarios = [
            scenario
            for scenario in plan["scenarios"]
            if scenario["suite"] == "seed_replay"
        ]
        self.assertEqual(
            {row["cell_key"] for row in seed_scenarios},
            {
                "mediation_bootstrap",
                "moderation_bootstrap",
            },
        )

    def test_recovery_moment_report_recomputes_bias_and_rmse(self) -> None:
        plan = runner.build_plan(
            qualification_trials=2,
            shard_size=2,
            included_suites=("recovery",),
        )
        scenario = next(
            row for row in plan["scenarios"] if row["cell_key"] == "mediation_point"
        )
        rows = [
            {
                "target_id": target_id,
                "n": 2,
                "failed_count": 0,
                "sum_error": 0.04,
                "sum_squared_error": 0.002,
            }
            for target_id in runner._recovery_target_ids("mediation")  # noqa: SLF001
        ]
        reports = runner._recovery_report(  # noqa: SLF001
            scenario,
            rows,
            completed=True,
            completed_trials=2,
            required_trials=2,
            thresholds=plan["frozen_thresholds"],
        )
        self.assertTrue(all(row["passed"] for row in reports))
        self.assertTrue(all(row["signed_bias"] == 0.02 for row in reports))
        self.assertTrue(all(abs(row["rmse"] - (0.001**0.5)) < 1e-12 for row in reports))

    def test_compact_ledger_rejects_inflated_denominator_and_full_gap(self) -> None:
        plan = runner.build_plan(
            qualification_trials=4,
            shard_size=4,
            included_suites=("failure_classification",),
        )
        with self.temporary_output() as raw:
            output = Path(raw)
            for shard in plan["shards"]:
                runner.execute_shard(plan, shard["shard_id"], output)
            aggregate = runner.aggregate_plan(plan, output)
            with mock.patch.object(runner, "build_plan", return_value=plan):
                runner.validate_frozen_full_aggregate(aggregate, plan)
                incomplete = runner._aggregate_from_compact_ledger(  # noqa: SLF001
                    plan, aggregate["shard_content_ledger"][:-1]
                )
                with self.assertRaises(runner.ArtifactTamperError):
                    runner.validate_frozen_full_aggregate(incomplete, plan)
            tampered = json.loads(json.dumps(aggregate["shard_content_ledger"][0]))
            metric = tampered["metric_counts"]["failure_classification_rate"]
            metric["eligible_count"] += 1
            shard = runner._shard_map(plan)[tampered["shard_id"]]  # noqa: SLF001
            scenario = runner._scenario_map(plan)[tampered["scenario_id"]]  # noqa: SLF001
            with self.assertRaises(runner.ArtifactTamperError):
                runner._validate_compact_shard_ledger_row(  # noqa: SLF001
                    tampered, plan=plan, shard=shard, scenario=scenario
                )

    def test_product_source_receipt_is_exact_and_tamper_rejected(self) -> None:
        receipt = runner._product_source_receipt()  # noqa: SLF001
        runner._validate_product_source_receipt(receipt)  # noqa: SLF001
        self.assertGreater(receipt["file_count"], 1)
        tampered = json.loads(json.dumps(receipt))
        tampered["files"][0]["sha256"] = "0" * 64
        tampered["source_set_sha256"] = runner.canonical_sha256(tampered["files"])
        with self.assertRaises(runner.ArtifactTamperError):
            runner._validate_product_source_receipt(tampered)  # noqa: SLF001

    def test_embedded_product_receipt_is_deeply_revalidated(self) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("current_product_comparison",),
        )
        shard = next(
            row
            for row in plan["shards"]
            if runner._scenario_map(plan)[row["scenario_id"]]["cell_key"]  # noqa: SLF001
            == "mediation_point"
        )
        definition = runner._scenario_map(plan)[shard["scenario_id"]]  # noqa: SLF001
        request = runner.build_product_request(plan, shard["shard_id"])
        source_receipt = runner._product_source_receipt()  # noqa: SLF001
        executable_sha = "1" * 64
        executable = {
            "path": runner._product_executable_path()  # noqa: SLF001
            .relative_to(runner.ROOT)
            .as_posix(),
            "size": 1,
            "sha256": executable_sha,
        }
        bundle_descriptor = {
            "path": (
                Path("validation/results/general_sem_rank0_qualification_v1")
                / "product_bundles"
                / f"{shard['shard_id']}.json"
            ).as_posix(),
            "size": 1,
            "sha256": "2" * 64,
        }
        nonce = "0123456789abcdef0123456789abcdef"
        receipt_body = {
            "schema_version": 2,
            "kind": runner.PRODUCT_EXECUTION_RECEIPT_KIND,
            "plan_sha256": plan["plan_sha256"],
            "shard_id": shard["shard_id"],
            "request_sha256": request["request_sha256"],
            "cargo_command": [
                "cargo",
                "run",
                "--locked",
                "-p",
                runner.PRODUCT_CARGO_PACKAGE,
                "--example",
                runner.PRODUCT_CARGO_EXAMPLE,
                "--",
                str(
                    (
                        runner.DEFAULT_OUTPUT_ROOT
                        / "product_requests"
                        / f"{shard['shard_id']}.json"
                    ).resolve()
                ),
                str(
                    (
                        runner.DEFAULT_OUTPUT_ROOT
                        / "product_bundles"
                        / f"{shard['shard_id']}.json"
                    ).resolve()
                ),
            ],
            "working_directory": str(runner.ROOT),
            "started_unix_ns": 1,
            "finished_unix_ns": 2,
            "cargo_exit_code": 0,
            "stdout_sha256": "3" * 64,
            "stderr_sha256": "4" * 64,
            "execution_nonce": nonce,
            "source_receipt": source_receipt,
            "executable_descriptor": executable,
            "bundle_descriptor": bundle_descriptor,
            "bundle_content_sha256": "5" * 64,
            "producer_executable_sha256": executable_sha,
            "maximum_available_workers": 8,
        }
        receipt = {
            **receipt_body,
            "receipt_sha256": runner.canonical_sha256(receipt_body),
        }
        production_receipts = {
            "production_result_sha256": "6" * 64,
            "compiled_plan_sha256": "7" * 64,
            "artifact_identity_sha256": "8" * 64,
            "requested_analytical_method_version": definition[
                "analytical_method_version"
            ],
            "point_analytical_method_version": definition["analytical_method_version"],
            "bootstrap": None,
        }
        observation = {
            "operation": request["operation"],
            "cell_id": definition["cell_id"],
            "method_version": definition["method_version"],
            "analytical_method_version": definition["analytical_method_version"],
            "scenario_seed": request["scenario_seed"],
            "bootstrap_seed": None,
            "bootstrap_index_plan_sha256": None,
            "cargo_invocation": (
                "cargo run --locked -p qpls-runner --example "
                "general_sem_rank0_product_comparison -- request output"
            ),
            "producer_executable_sha256": executable_sha,
            "maximum_available_workers": 8,
            "producer_contract_version": runner.PRODUCT_PRODUCER_CONTRACT,
            "execution_receipt_sha256": receipt["receipt_sha256"],
            "execution_nonce": nonce,
            "product_source_set_sha256": source_receipt["source_set_sha256"],
            "executable_descriptor": executable,
            "execution_receipt": receipt,
            "worker_comparisons": [
                {
                    "worker_axis": "not_applicable",
                    "workers": 1,
                    "product_result_sha256": "9" * 64,
                    "product_vs_python_difference_count": 0,
                    "product_vs_r_difference_count": 0,
                    "worker_replay_difference_count": 0,
                    "production_receipts": production_receipts,
                }
            ],
            "difference_count": 0,
            "maximum_absolute_difference": 0.0,
            "difference_witnesses": [],
            "python_result_sha256": "a" * 64,
            "r_result_sha256": "b" * 64,
            "r_runtime": {},
            "required_bindings": plan["current_product_comparison_contract"][
                "required_bindings"
            ],
        }
        runner._validate_embedded_product_observation(  # noqa: SLF001
            observation, plan=plan, shard=shard
        )
        tampered = json.loads(json.dumps(observation))
        tampered["analytical_method_version"] = "wrong_method_v1"
        with self.assertRaises(runner.ArtifactTamperError):
            runner._validate_embedded_product_observation(  # noqa: SLF001
                tampered, plan=plan, shard=shard
            )

    def test_product_ingest_requires_new_run_execution_receipt(self) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("current_product_comparison",),
        )
        with self.temporary_output() as raw:
            with self.assertRaises(runner.ArtifactTamperError):
                runner.ingest_product_bundle(
                    plan,
                    {},
                    Path(raw),
                    execution_receipt=None,
                )

    def test_product_runner_refuses_preexisting_bundle_without_launching_cargo(
        self,
    ) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("current_product_comparison",),
        )
        shard_id = plan["shards"][0]["shard_id"]
        with self.temporary_output() as raw:
            output = Path(raw)
            bundle = output / "product_bundles" / f"{shard_id}.json"
            bundle.parent.mkdir(parents=True)
            bundle.write_text("{}", encoding="utf-8")
            with self.assertRaises(runner.ArtifactTamperError):
                runner.run_product_comparison(plan, shard_id, output)

    def test_product_bootstrap_index_plan_is_exact_and_tamper_rejected(self) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("current_product_comparison",),
        )
        shard = next(
            row
            for row in plan["shards"]
            if runner._scenario_map(plan)[row["scenario_id"]]["cell_key"]  # noqa: SLF001
            == "mediation_bootstrap"
        )
        request = runner.build_product_request(plan, shard["shard_id"])
        definition = runner._scenario_map(plan)[shard["scenario_id"]]  # noqa: SLF001
        generated = runner.make_runner_scenario(
            definition, shard["trial_start_inclusive"]
        )
        cases = runner._product_complete_case_count(generated)  # noqa: SLF001
        requested = request["product_input"]["bootstrap"]["requested"]
        index_plan = [
            [index % cases for index in range(cases)] for _ in range(requested)
        ]
        bundle = {
            "bootstrap_index_plan": {
                "authority": "qpls_resampling::bootstrap_indices",
                "operation": "frozen-production-operation",
                "complete_case_count": cases,
                "requested": requested,
                "seed": request["bootstrap_seed"],
                "replicate_indices": index_plan,
            }
        }
        resolved = runner._validated_product_index_plan(  # noqa: SLF001
            bundle, request, generated
        )
        self.assertEqual(len(resolved), requested)
        tampered = json.loads(json.dumps(bundle))
        tampered["bootstrap_index_plan"]["replicate_indices"][0][0] = cases
        with self.assertRaises(runner.ArtifactTamperError):
            runner._validated_product_index_plan(tampered, request, generated)  # noqa: SLF001

    def test_product_bootstrap_authorities_preserve_point_primary_cells(self) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("current_product_comparison",),
        )
        expected = {
            "mediation_bootstrap": (
                ("qpls3.pls.mediation", "pls_mediation_v1"),
                (
                    "qpls3.pls.general_sem_multiple_mediation_bootstrap",
                    "general_sem_pls_full_model_case_bootstrap_v1",
                ),
            ),
            "moderation_bootstrap": (
                (
                    "qpls3.pls.general_sem_multiple_two_way_moderation_point",
                    "general_sem_pls_multiple_two_way_moderation_point_v1",
                ),
                (
                    "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
                    "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
                ),
            ),
        }
        definitions = {
            row["cell_key"]: row
            for row in plan["scenarios"]
            if row["suite"] == "current_product_comparison"
        }
        for cell_key, (primary_identity, supplemental_identity) in expected.items():
            with self.subTest(cell=cell_key):
                primary, supplemental = runner._expected_product_capability_authorities(  # noqa: SLF001
                    definitions[cell_key]
                )
                self.assertEqual(
                    (primary["cell_id"], primary["capability_version"]),
                    primary_identity,
                )
                self.assertIsNotNone(supplemental)
                self.assertEqual(
                    (
                        supplemental["cell_id"],
                        supplemental["capability_version"],
                    ),
                    supplemental_identity,
                )
                algorithm_method = runner._expected_product_analytical_method(  # noqa: SLF001
                    definitions[cell_key]
                )
                if cell_key == "mediation_bootstrap":
                    self.assertEqual(algorithm_method, supplemental_identity[1])
                else:
                    self.assertEqual(
                        algorithm_method,
                        "qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1",
                    )
                    self.assertNotEqual(algorithm_method, supplemental_identity[1])

        point_analytical = {
            "mediation_point": "pls_mediation_v1",
            "moderation_point": "qpls.general-sem-pls.multiple-two-way.point.v1",
        }
        for cell_key, method in point_analytical.items():
            with self.subTest(cell=cell_key):
                self.assertEqual(
                    runner._expected_product_analytical_method(  # noqa: SLF001
                        definitions[cell_key]
                    ),
                    method,
                )
                request_shard = next(
                    shard
                    for shard in plan["shards"]
                    if shard["scenario_id"] == definitions[cell_key]["scenario_id"]
                )
                request = runner.build_product_request(plan, request_shard["shard_id"])
                self.assertEqual(request["analytical_method_version"], method)

        tampered = json.loads(json.dumps(plan))
        tampered["scenarios"][0]["analytical_method_version"] = "wrong_method_v1"
        tampered["plan_sha256"] = runner.canonical_sha256(
            {key: value for key, value in tampered.items() if key != "plan_sha256"}
        )
        with self.assertRaises(runner.PlanValidationError):
            runner.validate_plan(tampered)

    def test_maximum_and_compound_are_deferred_to_exact_performance_receipts(
        self,
    ) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("maximum_axis", "compound_stress"),
        )
        delegated = [
            shard
            for shard in plan["shards"]
            if runner._scenario_map(plan)[shard["scenario_id"]]["suite"]  # noqa: SLF001
            in {"maximum_axis", "compound_stress"}
        ]
        self.assertEqual(len(delegated), 18)
        self.assertEqual(
            plan["external_performance_receipt_contract"][
                "required_hardware_profile_id"
            ],
            runner.PERFORMANCE_HARDWARE_PROFILE_ID,
        )
        with self.temporary_output() as raw:
            result = runner.execute_shard(plan, delegated[0]["shard_id"], Path(raw))
        self.assertEqual(result["status"], "requires_external_performance_receipt")
        required = plan["external_performance_receipt_contract"]["required_profiles"]
        self.assertEqual(
            required["maximum_axis"],
            [
                "maximum_rows_100000",
                "maximum_indicators_300",
                "maximum_constructs_100",
                "maximum_resamples_10000",
            ],
        )

    def test_invalid_external_performance_index_fails_closed(self) -> None:
        plan = runner.build_plan(
            qualification_trials=1,
            shard_size=1,
            included_suites=("maximum_axis", "compound_stress"),
        )
        with self.temporary_output() as raw:
            root = Path(raw)
            bad = root / "performance-index.json"
            bad.write_text("{}", encoding="utf-8")
            with self.assertRaises(runner.ArtifactTamperError):
                runner.ingest_external_performance_index(
                    plan, bad, root / "qualification"
                )

    def test_performance_workloads_resolve_from_each_cell_spec(self) -> None:
        cases = (
            ("mediation_point", "applied", "applied"),
            (
                "moderation_bootstrap",
                "maximum_axis",
                "maximum_rows_100000",
            ),
        )
        for cell_key, profile_id, case_id in cases:
            with self.subTest(cell=cell_key, profile=profile_id):
                contract = runner.CELL_CONTRACTS[cell_key]
                spec = runner.build_qualification_spec(cell_key)
                frozen = next(
                    row["workload"]
                    for row in spec["scenario_contract"]["complexity_profiles"]
                    if row["id"] == profile_id
                )
                body = {
                    "schema_version": 1,
                    "workload_kind": "general_sem_rank0_performance_workload",
                    "variant_id": cell_key,
                    "capability_reference": {
                        "registry_schema_version": 2,
                        "capability_id": contract.capability_id,
                        "cell_id": contract.cell_id,
                        "capability_version": contract.method_version,
                    },
                    "profile_id": profile_id,
                    "case_id": case_id,
                    "workload": dict(frozen),
                }
                row = {
                    **body,
                    "workload_fingerprint": runner.canonical_sha256(body),
                }
                runner._validate_cell_specific_performance_workload(row)  # noqa: SLF001
                tampered = json.loads(json.dumps(row))
                tampered["workload"]["groups"] = 2
                tampered_body = {
                    key: value
                    for key, value in tampered.items()
                    if key != "workload_fingerprint"
                }
                tampered["workload_fingerprint"] = runner.canonical_sha256(
                    tampered_body
                )
                with self.assertRaises(runner.PlanValidationError):
                    runner._validate_cell_specific_performance_workload(  # noqa: SLF001
                        tampered
                    )


if __name__ == "__main__":
    unittest.main()
