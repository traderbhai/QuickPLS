from __future__ import annotations

import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

import pls_algorithm_v1_current_source_oracle as oracle  # noqa: E402


class PlsAlgorithmV1CurrentSourceOracleTests(unittest.TestCase):
    def test_work_checks_are_non_promotional_and_deterministic(self) -> None:
        first = oracle.run_work_checks()
        second = oracle.run_work_checks()
        self.assertTrue(first["passed"])
        self.assertEqual(first, second)
        self.assertTrue(first["work_evidence_only"])
        self.assertFalse(first["qualification_ready"])
        self.assertFalse(first["promotion_requested"])
        self.assertLessEqual(
            max(first["metamorphic_max_abs_differences"].values()),
            first["metamorphic_tolerance"],
        )
        self.assertTrue(all(first["typed_boundaries"].values()))

    def test_hand_single_item_microcase_equals_correlation(self) -> None:
        rows = [[1.0, 1.0], [2.0, 3.0], [4.0, 4.0], [7.0, 8.0]]
        result = oracle.estimate_pls(
            rows,
            ["x1", "y1"],
            [
                oracle.OracleConstruct("x", ("x1",)),
                oracle.OracleConstruct("y", ("y1",)),
            ],
            [oracle.OraclePath("x", "y")],
        )
        observed = result["paths"][0]["coefficient"]
        expected = float(
            __import__("numpy").corrcoef(
                [row[0] for row in rows], [row[1] for row in rows]
            )[0, 1]
        )
        self.assertTrue(math.isclose(observed, expected, abs_tol=1.0e-12))

    def test_typed_cycle_and_nonconvergence_fail_closed(self) -> None:
        rows = [[1.0, 2.0], [2.0, 2.5], [3.0, 4.5], [5.0, 7.0]]
        constructs = [
            oracle.OracleConstruct("x", ("x1",)),
            oracle.OracleConstruct("y", ("y1",)),
        ]
        with self.assertRaisesRegex(oracle.PlsOracleFailure, "pls_cycle") as raised:
            oracle.estimate_pls(
                rows,
                ["x1", "y1"],
                constructs,
                [oracle.OraclePath("x", "y"), oracle.OraclePath("y", "x")],
            )
        self.assertEqual(raised.exception.code, "pls_cycle")

        fixture = oracle._fixture()  # noqa: SLF001 - frozen test fixture seam
        with self.assertRaisesRegex(
            oracle.PlsOracleFailure, "pls_non_convergence"
        ) as raised:
            oracle.estimate_pls(**fixture, max_iterations=1)
        self.assertEqual(raised.exception.code, "pls_non_convergence")

    def test_work_report_is_strict_json_and_reopenable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "oracle.json"
            expected = oracle.write_work_report(path)
            observed = json.loads(
                path.read_text(encoding="utf-8"),
                parse_constant=lambda value: (_ for _ in ()).throw(
                    ValueError(value)
                ),
            )
        self.assertEqual(observed, expected)


if __name__ == "__main__":
    unittest.main()
