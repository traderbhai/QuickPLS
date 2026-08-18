from __future__ import annotations

import csv
import math
import os
import random
import subprocess
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from pls_posthoc_technical_sample_size_contract import (  # noqa: E402
    ALPHA,
    FORMULA_CONSTANT,
    METHOD_VERSION,
    PathSignificance,
    POWER,
    SIGNIFICANCE_ALPHA,
    SIGNIFICANCE_ALTERNATIVE,
    SIGNIFICANCE_SOURCE,
    TEST_DIRECTION,
    StructuralPath,
    technical_minimum_sample_size,
)


class PlsPosthocTechnicalSampleSizeContractTests(unittest.TestCase):
    @staticmethod
    def _rscript_path() -> Path | None:
        configured = os.environ.get("QPLS_RSCRIPT")
        if configured and Path(configured).is_file():
            return Path(configured)
        documented = (
            Path.home()
            / "AppData/Local/Programs/R/R-4.6.1/bin/x64/Rscript.exe"
        )
        if documented.is_file():
            return documented
        return None

    def _run_r_oracle(
        self,
        paths: list[StructuralPath],
        inference: list[PathSignificance] | None,
        analytical_sample_size: int,
    ) -> dict[str, str]:
        rscript = self._rscript_path()
        if rscript is None:
            self.skipTest("Rscript is not installed; the release factory requires it")
        oracle = (
            VALIDATION
            / "oracles/pls_posthoc_technical_sample_size_v2_reference.R"
        )
        temporary_root = ROOT / "target" / "validation-temp"
        temporary_root.mkdir(parents=True, exist_ok=True)
        directory = temporary_root / f"qpls-posthoc-r-{os.getpid()}"
        directory.mkdir(parents=True, exist_ok=True)
        paths_path = directory / "paths.csv"
        inference_path = directory / "inference.csv"
        output_path = directory / "result.csv"
        with paths_path.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(
                handle, fieldnames=["source_id", "target_id", "coefficient"]
            )
            writer.writeheader()
            for path in paths:
                writer.writerow(
                    {
                        "source_id": path.source_id,
                        "target_id": path.target_id,
                        "coefficient": path.coefficient,
                    }
                )
        command = [
            str(rscript),
            str(oracle),
            "--paths",
            str(paths_path),
            "--analytical-n",
            str(analytical_sample_size),
            "--output",
            str(output_path),
        ]
        if inference is not None:
            with inference_path.open("w", encoding="utf-8", newline="") as handle:
                writer = csv.DictWriter(
                    handle,
                    fieldnames=[
                        "source_id",
                        "target_id",
                        "p_value_two_sided",
                    ],
                )
                writer.writeheader()
                for row in inference:
                    writer.writerow(
                        {
                            "source_id": row.source_id,
                            "target_id": row.target_id,
                            "p_value_two_sided": row.p_value_two_sided,
                        }
                    )
            command.extend(["--inference", str(inference_path)])
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        with output_path.open("r", encoding="utf-8", newline="") as handle:
            return next(csv.DictReader(handle))

    def test_published_formula_examples_and_fixed_assumptions(self):
        examples = {
            0.10: 619,
            0.15: 275,
            0.20: 155,
            0.25: 99,
            0.30: 69,
            0.40: 39,
        }
        self.assertEqual(FORMULA_CONSTANT, 2.486)
        self.assertEqual(ALPHA, 0.05)
        self.assertEqual(POWER, 0.80)
        self.assertEqual(TEST_DIRECTION, "directional")
        self.assertEqual(SIGNIFICANCE_ALTERNATIVE, "two_sided")
        self.assertEqual(SIGNIFICANCE_SOURCE, "pls_bootstrap_normal_reference_two_sided")
        self.assertNotEqual(TEST_DIRECTION, SIGNIFICANCE_ALTERNATIVE)
        for coefficient, expected in examples.items():
            with self.subTest(coefficient=coefficient):
                result = technical_minimum_sample_size(
                    [StructuralPath("source", "target", coefficient)],
                    [PathSignificance("source", "target", 0.01)],
                )
                self.assertEqual(result["status"], "available")
                self.assertEqual(result["required_sample_size"], expected)
                self.assertEqual(result["method_version"], METHOD_VERSION)

    def test_sign_and_declaration_order_do_not_change_driver_or_result(self):
        paths = [
            StructuralPath("z", "outcome", 0.20),
            StructuralPath("b", "outcome", -0.10),
            StructuralPath("a", "outcome", 0.10),
        ]
        inference = [
            PathSignificance(path.source_id, path.target_id, 0.01) for path in paths
        ]
        forward = technical_minimum_sample_size(paths, inference)
        reverse = technical_minimum_sample_size(reversed(paths), reversed(inference))
        self.assertEqual(forward, reverse)
        self.assertEqual(forward["required_sample_size"], 619)
        self.assertEqual(
            forward["driver"], StructuralPath("a", "outcome", 0.10)
        )

    def test_no_path_zero_path_range_and_nonfinite_behavior_are_typed(self):
        self.assertEqual(
            technical_minimum_sample_size([], None)["status"],
            "not_applicable_no_structural_path",
        )
        zero = technical_minimum_sample_size(
            [StructuralPath("source", "target", 0.0)],
            [PathSignificance("source", "target", 0.01)],
        )
        self.assertEqual(zero["status"], "undefined_zero_path")
        self.assertIsNone(zero["required_sample_size"])

        tiny = technical_minimum_sample_size(
            [StructuralPath("source", "target", 1e-300)],
            [PathSignificance("source", "target", 0.01)],
        )
        self.assertEqual(tiny["status"], "exceeds_supported_integer_range")
        self.assertIsNone(tiny["required_sample_size"])

        for value in (math.nan, math.inf, -math.inf):
            with self.subTest(value=value):
                result = technical_minimum_sample_size(
                    [StructuralPath("source", "target", value)],
                    [PathSignificance("source", "target", 0.01)],
                )
                self.assertEqual(result["status"], "inference_incomplete")
                self.assertIsNone(result["required_sample_size"])

        blank_identity = technical_minimum_sample_size(
            [StructuralPath(" ", "target", 0.20)],
            [PathSignificance(" ", "target", 0.01)],
        )
        self.assertEqual(blank_identity["status"], "inference_incomplete")

    def test_significance_is_required_complete_and_controls_driver_selection(self):
        paths = [
            StructuralPath("weak", "outcome", 0.10),
            StructuralPath("driver", "outcome", -0.20),
            StructuralPath("strong", "outcome", 0.60),
        ]
        unavailable = technical_minimum_sample_size(paths, None)
        self.assertEqual(unavailable["status"], "inference_unavailable")
        self.assertIsNone(unavailable["required_sample_size"])

        incomplete = technical_minimum_sample_size(
            paths,
            [PathSignificance("driver", "outcome", 0.01)],
        )
        self.assertEqual(incomplete["status"], "inference_incomplete")

        selected = technical_minimum_sample_size(
            paths,
            [
                PathSignificance("weak", "outcome", 0.20),
                PathSignificance("driver", "outcome", 0.05),
                PathSignificance("strong", "outcome", 0.001),
            ],
            analytical_sample_size=155,
        )
        self.assertEqual(selected["driver"], paths[1])
        self.assertEqual(selected["required_sample_size"], 155)
        self.assertEqual(selected["significant_path_count"], 2)
        self.assertTrue(selected["meets_technical_requirement"])

        none = technical_minimum_sample_size(
            paths,
            [
                PathSignificance(path.source_id, path.target_id, 0.051)
                for path in paths
            ],
        )
        self.assertEqual(none["status"], "no_statistically_significant_path")
        self.assertIsNone(none["driver"])

    def test_two_sided_significance_boundary_is_inclusive_without_changing_formula_direction(self):
        path = StructuralPath("source", "target", 0.20)
        included = technical_minimum_sample_size(
            [path],
            [PathSignificance("source", "target", SIGNIFICANCE_ALPHA)],
        )
        self.assertEqual(included["status"], "available")
        self.assertEqual(included["test_direction"], "directional")
        self.assertEqual(included["significance_alternative"], "two_sided")
        self.assertEqual(included["driver_p_value_two_sided"], 0.05)

        excluded = technical_minimum_sample_size(
            [path],
            [PathSignificance(
                "source",
                "target",
                math.nextafter(SIGNIFICANCE_ALPHA, math.inf),
            )],
        )
        self.assertEqual(excluded["status"], "no_statistically_significant_path")
        self.assertIsNone(excluded["driver"])

    def test_independent_r_oracle_matches_compact_seeded_contract_matrix(self):
        generator = random.Random(0x51504C53)
        probability_grid = [0.001, 0.01, 0.049, 0.05, 0.051, 0.20, 0.90]
        for scenario_index in range(24):
            path_count = 1 + scenario_index % 7
            paths = []
            inference = []
            for path_index in range(path_count):
                magnitude = generator.uniform(0.06, 0.85)
                coefficient = magnitude if generator.randrange(2) else -magnitude
                source = f"source-{path_index:02d}"
                target = f"target-{scenario_index % 3:02d}"
                paths.append(StructuralPath(source, target, coefficient))
                inference.append(
                    PathSignificance(
                        source,
                        target,
                        probability_grid[generator.randrange(len(probability_grid))],
                    )
                )
            generator.shuffle(paths)
            generator.shuffle(inference)
            analytical_n = 40 + scenario_index * 11
            python_result = technical_minimum_sample_size(
                paths, inference, analytical_sample_size=analytical_n
            )
            r_result = self._run_r_oracle(paths, inference, analytical_n)

            self.assertEqual(r_result["status"], python_result["status"])
            self.assertEqual(
                int(r_result["eligible_path_count"]),
                python_result["eligible_path_count"],
            )
            python_significant = python_result["significant_path_count"]
            self.assertEqual(
                r_result["significant_path_count"],
                "" if python_significant is None else str(python_significant),
            )
            driver = python_result["driver"]
            self.assertEqual(
                r_result["driver_source_id"], "" if driver is None else driver.source_id
            )
            self.assertEqual(
                r_result["driver_target_id"], "" if driver is None else driver.target_id
            )
            required = python_result["required_sample_size"]
            self.assertEqual(
                r_result["technically_required_sample_size"],
                "" if required is None else str(required),
            )
            if driver is not None:
                self.assertAlmostEqual(
                    float(r_result["minimum_absolute_path_coefficient"]),
                    abs(driver.coefficient),
                    places=14,
                )

        point_only_paths = [StructuralPath("source", "target", 0.20)]
        point_python = technical_minimum_sample_size(point_only_paths, None)
        point_r = self._run_r_oracle(point_only_paths, None, 0)
        self.assertEqual(point_r["status"], point_python["status"])

    def test_method_document_freezes_retrospective_caution_and_official_nuance(self):
        document = (
            ROOT / "docs/methods/PLS_POSTHOC_TECHNICAL_MINIMUM_SAMPLE_SIZE_V2.md"
        ).read_text(encoding="utf-8")
        for required in (
            "ceil((2.486 / abs(beta_min))^2)",
            "alpha = 0.05",
            "power `= 0.80`",
            "directional test",
            "retrospective",
            "statistically significant",
            "smallest absolute statistically significant",
            "inference_unavailable",
            "two-sided normal-reference",
            "undefined_zero_path",
            "qpls3.pls.sample_size_power",
        ):
            with self.subTest(required=required):
                self.assertIn(required, document)


if __name__ == "__main__":
    unittest.main()
