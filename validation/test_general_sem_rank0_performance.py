from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from validation import general_sem_rank0_performance as performance
from validation.general_sem_rank0_packaged_acceptance import ContractError
from validation.general_sem_rank0_performance import (
    HARDWARE_PROFILE_ID,
    PROFILE_CASES,
    _validate_observations,
    build_plan,
    canonical_sha256,
    workload_fingerprint_authority,
)


ROOT = Path(__file__).resolve().parent.parent


class GeneralSemRank0PerformanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest, cls.context, cls.rows = build_plan()

    def test_plan_is_exact_applicable_thirty_cell_matrix(self) -> None:
        self.assertEqual(HARDWARE_PROFILE_ID, "standard_windows_6c16g")
        self.assertEqual(len(self.rows), 30)
        self.assertEqual(
            PROFILE_CASES,
            (
                ("micro_exact", "micro_exact"),
                ("applied", "applied"),
                ("large", "large"),
                ("maximum_axis", "maximum_rows_100000"),
                ("maximum_axis", "maximum_indicators_300"),
                ("maximum_axis", "maximum_constructs_100"),
                ("maximum_axis", "maximum_resamples_10000"),
                ("compound_stress", "compound_stress"),
            ),
        )
        self.assertEqual(len({row["variant_id"] for row in self.rows}), 4)
        self.assertEqual(
            len(
                {
                    (row["variant_id"], row["profile_id"], row["case_id"])
                    for row in self.rows
                }
            ),
            30,
        )
        point_max_resamples = [
            row
            for row in self.rows
            if row["variant_id"] in {
                "mediation_point",
                "multiple_two_way_moderation_point",
            }
            and row["case_id"] == "maximum_resamples_10000"
        ]
        self.assertEqual(point_max_resamples, [])
        self.assertEqual(
            len(
                [
                    row
                    for row in self.rows
                    if row["profile_id"]
                    in {"maximum_axis", "compound_stress"}
                ]
            ),
            18,
        )
        self.assertTrue(
            all(len(row["workload_fingerprint"]) == 64 for row in self.rows)
        )
        self.assertEqual(
            {
                (row["variant_id"], row["mandatory_combination_id"])
                for row in self.rows
                if row["case_id"] == "large"
            },
            {
                ("mediation_point", "large_point_replay"),
                ("multiple_mediation_bootstrap", "large_worker_replay"),
                (
                    "multiple_two_way_moderation_point",
                    "large_point_replay",
                ),
                (
                    "multiple_two_way_moderation_bootstrap",
                    "large_worker_replay",
                ),
            },
        )

    def test_each_workload_is_exactly_spec_derived_and_fingerprinted(self) -> None:
        dimensions = {
            "micro_exact": (80, 15, 5),
            "applied": (1_000, 40, 10),
            "large": (10_000, 80, 20),
            "maximum_rows_100000": (100_000, 40, 10),
            "maximum_indicators_300": (2_000, 300, 20),
            "maximum_constructs_100": (5_000, 300, 100),
            "maximum_resamples_10000": (1_000, 40, 10),
            "compound_stress": (25_000, 150, 50),
        }
        bootstrap_variants = {
            "multiple_mediation_bootstrap",
            "multiple_two_way_moderation_bootstrap",
        }
        for row in self.rows:
            workload = row["workload"]
            rows, indicators, constructs = dimensions[row["case_id"]]
            expected_resamples = (
                0
                if row["variant_id"] not in bootstrap_variants
                else 199
                if row["profile_id"] == "micro_exact"
                else 5_000
                if row["profile_id"] in {"applied", "large"}
                else 10_000
            )
            self.assertEqual(
                workload,
                {
                    "rows": rows,
                    "indicators": indicators,
                    "constructs": constructs,
                    "resamples": expected_resamples,
                    "groups": 1,
                    "candidate_models": 1,
                },
            )
            authority = workload_fingerprint_authority(row)
            self.assertEqual(row["workload_fingerprint"], canonical_sha256(authority))
            descriptor = row["qualification_spec"]
            self.assertEqual(
                set(descriptor),
                {
                    "path",
                    "qualification_id",
                    "spec_frozen_at_utc",
                    "qualification_contract_sha256",
                },
            )
            spec_path = ROOT / descriptor["path"]
            spec_bytes = spec_path.read_bytes()
            spec = json.loads(spec_bytes)
            self.assertEqual(
                descriptor["qualification_id"], spec["identity"]["qualification_id"]
            )
            self.assertEqual(
                descriptor["spec_frozen_at_utc"], "2026-08-19T00:00:00Z"
            )
            self.assertEqual(
                descriptor["qualification_contract_sha256"],
                performance.qualification_contract_sha256(spec),
            )

    def test_workload_fingerprints_are_stable_across_accepted_and_current_plans(self) -> None:
        _, _, repeated = build_plan()
        self.assertEqual(
            [row["workload_fingerprint"] for row in self.rows],
            [row["workload_fingerprint"] for row in repeated],
        )

    def test_workload_fingerprint_binds_normalized_contract_not_mutable_raw_spec_bytes(self) -> None:
        row = copy.deepcopy(self.rows[0])
        fingerprint = canonical_sha256(workload_fingerprint_authority(row))
        spec_path = ROOT / row["qualification_spec"]["path"]
        specification = json.loads(spec_path.read_text(encoding="utf-8"))
        mutated = copy.deepcopy(specification)
        mutated["evidence_contract"]["receipts"] = [{"mutable": "diagnostic"}]
        self.assertEqual(
            performance.qualification_contract_sha256(mutated),
            performance.qualification_contract_sha256(specification),
        )
        self.assertEqual(canonical_sha256(workload_fingerprint_authority(row)), fingerprint)
        row["qualification_spec"]["qualification_contract_sha256"] = "1" * 64
        self.assertNotEqual(
            canonical_sha256(workload_fingerprint_authority(row)), fingerprint
        )

    def test_applied_observation_requires_atomic_cancel_and_ten_run_memory_soak(self) -> None:
        row = next(
            row
            for row in self.rows
            if row["variant_id"] == "mediation_point"
            and row["profile_id"] == "applied"
        )
        observation = {
            "cancellation_observation": {
                "terminal_latency_seconds": 0.25,
                "terminal_state": "cancelled",
                "no_partial_visible_result": True,
                "no_partial_committed_result": True,
                "archive_unchanged": True,
            },
            "memory_growth_observation": {
                "accepted_runs": 10,
                "first_settled_working_set_bytes": 1_000,
                "last_settled_working_set_bytes": 1_040,
                "growth_percent": 4,
                "orphan_processes": 0,
            },
        }
        _validate_observations(observation, row, self.manifest)

        slow = copy.deepcopy(observation)
        slow["cancellation_observation"]["terminal_latency_seconds"] = 1.001
        with self.assertRaisesRegex(ContractError, "cancellation"):
            _validate_observations(slow, row, self.manifest)

        leaking = copy.deepcopy(observation)
        leaking["memory_growth_observation"]["growth_percent"] = 5.001
        with self.assertRaisesRegex(ContractError, "memory"):
            _validate_observations(leaking, row, self.manifest)

    def test_micro_case_rejects_fabricated_cancel_or_memory_observations(self) -> None:
        row = next(
            row
            for row in self.rows
            if row["variant_id"] == "mediation_point"
            and row["profile_id"] == "micro_exact"
        )
        _validate_observations({}, row, self.manifest)
        with self.assertRaisesRegex(ContractError, "fields are not exact"):
            _validate_observations(
                {
                    "cancellation_observation": {
                        "terminal_latency_seconds": 0.1
                    }
                },
                row,
                self.manifest,
            )

    def test_cell_performance_validator_is_atomic_across_unrelated_failures(self) -> None:
        target_reference = self.rows[0]["capability_reference"]
        target_plan = [
            row for row in self.rows if row["capability_reference"] == target_reference
        ]
        rank0_contract = performance.load_json(performance.DEFAULT_RANK0_CONTRACT)
        rank0_context = performance.validate_rank0_contract(
            rank0_contract,
            performance.load_json(performance.DEFAULT_REGISTRY_PATH),
        )
        cases = [
            {
                "variant_id": row["variant_id"],
                "capability_reference": row["capability_reference"],
                "profile_id": row["profile_id"],
                "case_id": row["case_id"],
                "workload_fingerprint": row["workload_fingerprint"],
                "warmup_runs": 1,
                "measured_runs": 5,
                "cancellation_observed": bool(row["potentially_long"]),
                "memory_soak_accepted_runs": 10 if row["repeat_memory_gate"] else 0,
                "receipt": {
                    "path": f"receipts/{row['case_id']}.json",
                    "size": 1,
                    "sha256": "a" * 64,
                },
                "baseline": None,
                "receipt_complete": True,
            }
            for row in target_plan
        ]
        cases.append(
            {
                "capability_reference": self.rows[-1]["capability_reference"],
                "receipt_complete": False,
            }
        )
        index = {
            "schema_version": 1,
            "evidence_kind": "general_sem_rank0_method_performance_index",
            "measurement_role": "current",
            "contract_id": self.manifest["contract_id"],
            "contract_sha256": self.context["contract_sha256"],
            "rank0_contract_sha256": canonical_sha256(rank0_contract),
            "hardware_profile_id": HARDWARE_PROFILE_ID,
            "hardware_fingerprint": {
                "os": "windows_11",
                "architecture": "x86_64",
                "cpu": "Fixture CPU",
                "physical_cores": 6,
                "logical_cores": 12,
                "memory_bytes": 16 * 1024**3,
            },
            "build_fingerprint": "b" * 64,
            "source_receipt": performance.unified_rank0_source_receipt(ROOT),
            "qualification_contracts": performance.qualification_contract_authorities(
                rank0_context, ROOT
            ),
            "generated_at_utc": "2026-08-19T00:00:00Z",
            "case_count": len(cases),
            "cases": cases,
            "cell_pass_ledger": [
                {
                    "capability_reference": target_reference,
                    "expected_case_count": len(target_plan),
                    "accepted_case_count": len(target_plan),
                    "passed": True,
                },
                {
                    "capability_reference": self.rows[-1]["capability_reference"],
                    "expected_case_count": 8,
                    "accepted_case_count": 7,
                    "passed": False,
                },
            ],
            "passed": False,
        }
        result = performance.validate_cell_performance_index(
            index, target_reference, repository_root=ROOT
        )
        self.assertIs(result["passed"], True)
        self.assertEqual(result["case_count"], len(target_plan))
        index["cases"][0]["receipt_complete"] = False
        with self.assertRaisesRegex(ContractError, "incomplete or differs"):
            performance.validate_cell_performance_index(
                index, target_reference, repository_root=ROOT
            )


if __name__ == "__main__":
    unittest.main()
