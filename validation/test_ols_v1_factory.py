import copy
import json
import math
import sys
import unittest
from unittest import mock
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from method_promotion_manifest import validate_manifest  # noqa: E402
from ols_v1_reference import compare_quickpls, fit_ols_hc3  # noqa: E402


MANIFEST = ROOT / "validation/methods/ols_v1.manifest.json"
FACTORY = ROOT / "validation/results/method_factory/ols_v1"


class OlsV1FactoryTests(unittest.TestCase):
    def test_independent_reference_recovers_orthogonal_hand_fixture(self):
        rows = [
            {"y": 1.25 + 0.75 * x - 0.5 * z + 0.1 * x * z, "x": x, "z": z}
            for x in (-3.0, -1.0, 1.0, 3.0)
            for z in (-1.0, 1.0)
        ]
        reference = fit_ols_hc3(rows, "y", ["x", "z"])
        self.assertEqual(reference["terms"], ["intercept", "x", "z"])
        for actual, expected in zip(reference["coefficients"], [1.25, 0.75, -0.5]):
            self.assertAlmostEqual(actual, expected, places=12)
        self.assertTrue(all(math.isfinite(value) for value in reference["standard_errors"]))

    def test_reference_comparator_rejects_scientific_tampering(self):
        rows = [
            {"y": 1.0 + 0.4 * index + (index % 3) * 0.1, "x": float(index)}
            for index in range(1, 20)
        ]
        reference = fit_ols_hc3(rows, "y", ["x"])
        regression = {
            "coefficients": [
                {
                    "term": term,
                    "estimate": reference["coefficients"][index],
                    "standard_error": reference["standard_errors"][index],
                    "statistic": reference["statistics"][index],
                    "p_value_two_sided": reference["p_values"][index],
                    "confidence_interval_lower": reference["confidence_interval_lower"][index],
                    "confidence_interval_upper": reference["confidence_interval_upper"][index],
                }
                for index, term in enumerate(reference["terms"])
            ],
            "fit": copy.deepcopy(reference["fit"]),
            "predictions": [
                {"fitted": fitted, "residual": residual}
                for fitted, residual in zip(reference["fitted"], reference["residuals"])
            ],
        }
        self.assertTrue(compare_quickpls(regression, reference)["passed"])
        regression["coefficients"][1]["estimate"] += 0.01
        self.assertFalse(compare_quickpls(regression, reference)["passed"])

    def test_reference_fails_closed_on_ols_boundaries_and_counts_missing_rows(self):
        cases = {
            "high_leverage": (
                [
                    {"y": y, "x": x}
                    for y, x in [(0.0, 0.0), (0.2, 0.0), (-0.1, 0.0), (1.0, 1.0)]
                ],
                ["x"],
                "HC3 leverage",
            ),
            "constant_predictor": (
                [{"y": float(index), "x": 1.0} for index in range(4)],
                ["x"],
                "rank-deficient",
            ),
            "nonpositive_residual_df": (
                [
                    {"y": 0.0, "x": 0.0, "z": 0.0},
                    {"y": 1.0, "x": 1.0, "z": 0.0},
                    {"y": 2.0, "x": 0.0, "z": 1.0},
                ],
                ["x", "z"],
                "positive residual degrees",
            ),
        }
        for name, (rows, predictors, message) in cases.items():
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, message):
                fit_ols_hc3(rows, "y", predictors)

        valid_rows = [
            {"y": 1.0, "x": 0.0},
            {"y": 2.1, "x": 1.0},
            {"y": "", "x": 2.0},
            {"y": 4.0, "x": ""},
            {"y": 4.8, "x": 4.0},
            {"y": 6.2, "x": 5.0},
        ]
        reference = fit_ols_hc3(valid_rows, "y", ["x"])
        self.assertEqual(reference["observations"], 4)
        self.assertEqual(reference["omitted_observations"], 2)

        complete_rows = [
            {"y": 1.0 + 0.4 * index + (index % 3) * 0.1, "x": float(index)}
            for index in range(1, 20)
        ]
        for diagonal in ([0.0, 1.0], [float("nan"), 1.0]):
            with (
                self.subTest(covariance_diagonal=diagonal),
                mock.patch("ols_v1_reference.np.diag", return_value=np.asarray(diagonal)),
                self.assertRaisesRegex(ValueError, "finite and strictly positive"),
            ):
                fit_ols_hc3(complete_rows, "y", ["x"])

    def test_manifest_derives_release_state_from_current_evidence(self):
        result = validate_manifest(MANIFEST, ROOT)
        self.assertTrue(result["passed"], result["errors"])
        self.assertEqual(result["declared_state"], "release_qualified")
        self.assertEqual(result["derived_state"], "release_qualified")
        self.assertEqual(
            [row["state"] for row in result["stage_results"] if row["passed"]][:3],
            ["engine_only", "archive_qualified", "native_qualified"],
        )

    def test_evidence_is_method_specific_and_release_contract_is_exact(self):
        document = json.loads(MANIFEST.read_text(encoding="utf-8"))
        evidence = document["qualification"]["evidence"]
        self.assertEqual(len(evidence["engine_only"]), 1)
        self.assertEqual(len(evidence["archive_qualified"]), 1)
        self.assertEqual(len(evidence["native_qualified"]), 1)
        self.assertEqual(
            sorted(role for row in evidence["release_qualified"] for role in row["roles"]),
            ["method_audit", "packaged_acceptance"],
        )
        for filename in (
            "engine_stage.identity.json",
            "persistence_report.identity.json",
            "native_stage.identity.json",
        ):
            report = json.loads((FACTORY / filename).read_text(encoding="utf-8"))
            self.assertTrue(report["passed"])
            self.assertEqual(report["feature_id"], "qpls3.standalone.ols")
            self.assertEqual(report["method_version"], "regression_ols_v1")
            serialized = json.dumps(report, sort_keys=True).lower()
            self.assertNotIn("cumulative_native_acceptance", serialized)
            self.assertNotIn("packaged_acceptance.identity", serialized)


if __name__ == "__main__":
    unittest.main()
