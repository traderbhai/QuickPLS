#!/usr/bin/env python3
"""Focused source-level tests for the independent randomization reference."""

from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

import numpy as np


VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

import structural_path_randomization_reference as reference


class StructuralPathRandomizationReferenceTests(unittest.TestCase):
    def test_chacha20_zero_seed_matches_standard_first_block_prefix(self) -> None:
        block = reference.chacha20_block(bytes(32))
        self.assertEqual(block[:16].hex(), "76b8e0ada0f13d90405d6ae55386bd28")

    def test_frozen_corporate_index_vectors_match_0_and_98(self) -> None:
        parameter = reference.canonical_parameter("comp", "satisfaction")
        operation = reference.operation_for_parameter(parameter)
        for index, expected in reference.FROZEN_INDEX_VECTORS.items():
            actual = reference.permutation_indices(12, reference.SEED, operation, index)
            self.assertEqual(actual, expected)
            self.assertEqual(sorted(actual), list(range(12)))

    def test_schema_v3_migration_is_typed_and_keeps_exact_path_order(self) -> None:
        recipe = reference.migrated_recipe_payload(4)
        self.assertEqual(
            reference.FEATURE_ID,
            "qpls3.inference.structural_path_randomization",
        )
        self.assertEqual(reference.CATALOGUE_SNAPSHOT_DATE, "2026-08-12")
        self.assertEqual(reference.ANALYSIS_RESULT_SCHEMA_VERSION, 1)
        self.assertEqual(reference.ANALYSIS_RECIPE_SCHEMA_VERSION, 3)
        self.assertEqual(reference.PLS_INFERENCE_PAYLOAD_KIND, "pls_pm_v3")
        self.assertEqual(reference.PROJECT_ARCHIVE_SCHEMA_VERSION, 5)
        self.assertEqual(recipe["schema_version"], reference.ANALYSIS_RECIPE_SCHEMA_VERSION)
        self.assertEqual(recipe["method_config"], {"kind": "pls_permutation"})
        self.assertEqual(recipe["settings"]["workers"], 4)
        self.assertEqual(recipe["settings"]["permutation_samples"], reference.PERMUTATIONS)
        self.assertEqual(recipe["settings"]["bootstrap_samples"], 0)
        self.assertEqual(
            [(row["source"], row["target"]) for row in recipe["model"]["paths"]],
            list(reference.EXPECTED_PATHS),
        )
        self.assertEqual(
            reference.expected_parameter_manifest(),
            [
                '["path",["comp","satisfaction"]]',
                '["path",["like","satisfaction"]]',
                '["path",["satisfaction","loyalty"]]',
            ],
        )

    def test_release_cli_freshness_set_covers_every_critical_crate_family(self) -> None:
        paths = {
            path.relative_to(reference.ROOT).as_posix()
            for path in reference.release_cli_freshness_sources()
        }
        self.assertTrue({"Cargo.toml", "Cargo.lock"}.issubset(paths))
        for crate in (
            "qpls-core",
            "qpls-data",
            "qpls-assessment",
            "qpls-estimation",
            "qpls-resampling",
            "qpls-runner",
            "qpls-project",
            "qpls-cli",
        ):
            prefix = f"crates/{crate}/"
            self.assertIn(f"{prefix}Cargo.toml", paths)
            self.assertTrue(
                any(path.startswith(prefix) and path.endswith(".rs") for path in paths),
                crate,
            )
        cli_manifest = (reference.ROOT / "crates" / "qpls-cli" / "Cargo.toml").read_text(
            encoding="utf-8"
        )
        self.assertIn('qpls-project = { path = "../qpls-project" }', cli_manifest)

    def test_reference_source_manifest_binds_executing_and_audit_sources(self) -> None:
        sources = reference.reference_evidence_sources()
        relative = [path.relative_to(reference.ROOT).as_posix() for path in sources]
        self.assertEqual(relative, list(reference.REFERENCE_EVIDENCE_SOURCE_PATHS))
        self.assertEqual(sources[0], Path(reference.__file__).resolve())
        self.assertIn(
            "validation/structural_path_randomization_method_promotion_audit.py",
            relative,
        )
        self.assertIn("validation/r_runtime.py", relative)
        self.assertIn("validation/inference_publication_audit.py", relative)

    def test_multi_predictor_freedman_lane_hand_fixture(self) -> None:
        focal = np.asarray([-1, -1, -1, -1, 1, 1, 1, 1], dtype=float)
        nuisance_a = np.asarray([-1, -1, 1, 1, -1, -1, 1, 1], dtype=float)
        nuisance_b = np.asarray([-1, 1, -1, 1, -1, 1, -1, 1], dtype=float)
        outcome = 2 * focal + 3 * nuisance_a + 5 * nuisance_b + focal * nuisance_a
        estimate = reference.freedman_lane_focal_coefficient(
            [focal, nuisance_a, nuisance_b], outcome, 0, list(reversed(range(8)))
        )
        self.assertAlmostEqual(estimate, -2.0, places=12)
        with self.assertRaisesRegex(ValueError, "bijection"):
            reference.freedman_lane_focal_coefficient(
                [focal, nuisance_a, nuisance_b], outcome, 0, [0, 1, 2, 3, 4, 5, 6, 6]
            )

    def test_cli_contract_enforces_exact_order_original_and_plus_one_identity(self) -> None:
        recipe = reference.migrated_recipe_payload(1)
        scores = {
            construct["id"]: [float(index) for index in range(12)]
            for construct in recipe["model"]["constructs"]
        }
        paths = [
            {"source": source, "target": target, "coefficient": 0.1 + ordinal}
            for ordinal, (source, target) in enumerate(reference.EXPECTED_PATHS)
        ]
        parameters = [
            {
                "parameter": reference.canonical_parameter(row["source"], row["target"]),
                "original": row["coefficient"],
                "exceedances": ordinal,
                "p_value_two_sided": (ordinal + 1.0) / (reference.PERMUTATIONS + 1.0),
                "permutations": reference.PERMUTATIONS,
            }
            for ordinal, row in enumerate(paths)
        ]
        result = {
            "schema_version": reference.ANALYSIS_RESULT_SCHEMA_VERSION,
            "id": "result",
            "status": "completed",
            "diagnostics": [],
            "provenance": {
                "recipe_id": recipe["id"],
                "dataset_fingerprint": recipe["dataset_fingerprint"],
                "method": "pls_pm",
                "method_version": reference.EXPECTED_PROVENANCE_METHOD_VERSION,
                "seed": reference.SEED,
                "settings": {
                    "workers": 1,
                    "permutation_samples": reference.PERMUTATIONS,
                },
            },
            "payload": {
                "kind": reference.PLS_INFERENCE_PAYLOAD_KIND,
                "estimation": {
                    "paths": paths,
                    "construct_scores": dict(sorted(scores.items())),
                    "used_observations": 12,
                    "omitted_observations": 0,
                },
                "assessment": {},
                "permutation": {
                    "method_version": reference.METHOD_VERSION,
                    "plan": {
                        "permutations": reference.PERMUTATIONS,
                        "master_seed": reference.SEED,
                        "operation": reference.PLAN_OPERATION,
                    },
                    "parameters": parameters,
                },
            },
        }
        validated = reference.validate_cli_contract(result, 1, recipe)
        self.assertEqual(len(validated["parameters"]), 3)
        self.assertEqual(
            validated["result_schema_version"],
            reference.ANALYSIS_RESULT_SCHEMA_VERSION,
        )
        self.assertEqual(validated["recipe_id"], recipe["id"])
        self.assertEqual(
            validated["dataset_fingerprint"], recipe["dataset_fingerprint"]
        )

        wrong_provenance = copy.deepcopy(result)
        wrong_provenance["provenance"]["method_version"] = (
            f"pls_pm_v1+{reference.METHOD_VERSION}"
        )
        with self.assertRaisesRegex(ValueError, "method-version chain"):
            reference.validate_cli_contract(wrong_provenance, 1, recipe)

        for wrong_version in (0, 2, 3, 4, reference.PROJECT_ARCHIVE_SCHEMA_VERSION):
            wrong_schema = copy.deepcopy(result)
            wrong_schema["schema_version"] = wrong_version
            with self.subTest(wrong_version=wrong_version):
                with self.assertRaisesRegex(ValueError, "result envelope.*schema v1"):
                    reference.validate_cli_contract(wrong_schema, 1, recipe)

        wrong_payload_kind = copy.deepcopy(result)
        wrong_payload_kind["payload"]["kind"] = "pls_pm_v1"
        with self.assertRaisesRegex(ValueError, "payload is not pls_pm_v3"):
            reference.validate_cli_contract(wrong_payload_kind, 1, recipe)

        wrong_recipe = copy.deepcopy(result)
        wrong_recipe["provenance"]["recipe_id"] = "different-recipe"
        with self.assertRaisesRegex(ValueError, "recipe ID"):
            reference.validate_cli_contract(wrong_recipe, 1, recipe)

        wrong_fingerprint = copy.deepcopy(result)
        wrong_fingerprint["provenance"]["dataset_fingerprint"] = "v2:wrong"
        with self.assertRaisesRegex(ValueError, "dataset fingerprint"):
            reference.validate_cli_contract(wrong_fingerprint, 1, recipe)

        reordered = copy.deepcopy(result)
        reordered["payload"]["permutation"]["parameters"].reverse()
        with self.assertRaisesRegex(ValueError, "ordered path manifest"):
            reference.validate_cli_contract(reordered, 1, recipe)

        bad_probability = copy.deepcopy(result)
        bad_probability["payload"]["permutation"]["parameters"][0][
            "p_value_two_sided"
        ] = 0.0010000000000000002
        with self.assertRaisesRegex(ValueError, "probability identity"):
            reference.validate_cli_contract(bad_probability, 1, recipe)

    def test_worker_comparison_is_exact_and_fail_closed(self) -> None:
        base = {
            "payload": {"permutation": {"parameters": [{"p": 0.01}]}},
            "provenance": {
                "recipe_id": "serial",
                "started_at": "a",
                "completed_at": "b",
                "settings": {"workers": 1, "seed": reference.SEED},
            },
        }
        parallel = copy.deepcopy(base)
        parallel["provenance"].update(
            {"recipe_id": "parallel", "started_at": "c", "completed_at": "d"}
        )
        parallel["provenance"]["settings"]["workers"] = 4
        reference.assert_worker_invariance(base, parallel)
        parallel["payload"]["permutation"]["parameters"][0]["p"] = float.fromhex(
            float(0.01).hex()
        ) + np.spacing(0.01)
        with self.assertRaisesRegex(ValueError, "differs"):
            reference.assert_worker_invariance(base, parallel)

        signed_zero = copy.deepcopy(base)
        signed_zero_parallel = copy.deepcopy(parallel)
        signed_zero["payload"] = {"value": -0.0}
        signed_zero_parallel["payload"] = {"value": 0.0}
        with self.assertRaisesRegex(ValueError, "f64 bits"):
            reference.assert_worker_invariance(signed_zero, signed_zero_parallel)

        numeric_type = copy.deepcopy(base)
        numeric_type_parallel = copy.deepcopy(parallel)
        numeric_type["payload"] = {"value": 1}
        numeric_type_parallel["payload"] = {"value": 1.0}
        with self.assertRaisesRegex(ValueError, "type differs"):
            reference.assert_worker_invariance(numeric_type, numeric_type_parallel)

        reordered = copy.deepcopy(base)
        reordered_parallel = copy.deepcopy(base)
        reordered["payload"] = {"first": 1, "second": 2}
        reordered_parallel["payload"] = {"second": 2, "first": 1}
        with self.assertRaisesRegex(ValueError, "key/order differs"):
            reference.assert_worker_invariance(reordered, reordered_parallel)

    def test_calibration_thresholds_are_predeclared_and_fail_closed(self) -> None:
        lower, upper = reference.wilson_interval(12, 240)
        self.assertLessEqual(lower, reference.CALIBRATION_ALPHA)
        self.assertGreaterEqual(upper, reference.CALIBRATION_ALPHA)
        null_limit = reference.CALIBRATION_ALPHA + reference.CALIBRATION_NULL_Z * (
            reference.CALIBRATION_ALPHA
            * (1.0 - reference.CALIBRATION_ALPHA)
            / reference.CALIBRATION_REPLICATIONS
        ) ** 0.5
        self.assertLess(null_limit, 0.10)
        self.assertEqual(reference.CALIBRATION_POWER_MINIMUM, 0.80)
        self.assertEqual(
            reference.CALIBRATION_NOISE_DISTRIBUTION,
            "homoscedastic_standard_gaussian",
        )
        self.assertEqual(reference.CALIBRATION_VARIANCE_MODEL, "constant")

    def test_parameter_identity_parser_rejects_non_path_shapes(self) -> None:
        self.assertEqual(
            reference._parameter_path('["path",["comp","satisfaction"]]'),
            ("comp", "satisfaction"),
        )
        for invalid in ('["path",["comp"]]', '["effect",["a","b"]]', "not-json"):
            with self.assertRaises((ValueError, json.JSONDecodeError)):
                reference._parameter_path(invalid)


if __name__ == "__main__":
    unittest.main()
