#!/usr/bin/env python3
"""Focused transport-contract tests for bounded metamorphic orchestration.

These tests use tiny fake producer files and reports. They never invoke Cargo,
the Rust examples, or the scientific verifier.
"""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace


HERE = Path(__file__).resolve().parent
MODULE_PATH = HERE / "multimod_metamorphic_cells_v1.py"
SPEC = importlib.util.spec_from_file_location("metamorphic_cells", MODULE_PATH)
assert SPEC and SPEC.loader
cells = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(cells)


class MetamorphicPlanContractTests(unittest.TestCase):
    def test_plan_is_exact_and_baseline_rooted(self) -> None:
        plan = cells.plan_document()
        rows = plan["cells"]
        self.assertEqual(25, len(rows))
        self.assertEqual(25, len({row["cell_id"] for row in rows}))
        self.assertEqual(
            [
                "mga-baseline",
                "heterogeneity-baseline",
                "conditional-baseline",
                "causal-baseline",
            ],
            [row["cell_id"] for row in rows[:4]],
        )
        for row in rows[:4]:
            self.assertEqual([], row["dependencies"])
        for row in rows[4:]:
            self.assertEqual([f"{row['family']}-baseline"], row["dependencies"])
        self.assertEqual(4, plan["maximum_parallel_cells"])
        self.assertEqual(1800, plan["maximum_cell_seconds"])
        self.assertEqual(6480, plan["maximum_scientific_seconds"])
        self.assertEqual(6600, plan["maximum_wrapper_seconds"])
        self.assertFalse(plan["scientific_settings_changed"])
        self.assertEqual("development", plan["scientific_scale"])
        sign = next(row for row in rows if row["axis"] == "sign_reverse")
        self.assertEqual("causal", sign["family"])
        self.assertEqual("c", sign["sign_columns"])
        self.assertTrue(
            all(
                row["sign_columns"] == ""
                for row in rows
                if row["axis"] != "sign_reverse"
            )
        )

    def test_seal_cli_preserves_option_like_direct_executable_arguments(self) -> None:
        parsed = cells.parser().parse_args(
            [
                "seal",
                "--plan",
                "plan.json",
                "--cell-dir",
                "cells",
                "--cell-id",
                "mga-baseline",
                "--source-commit",
                "1" * 40,
                "--executable",
                "producer.exe",
                "--executable-sha256",
                "2" * 64,
                "--temporary-result",
                "mga-baseline.result.tmp.json",
                "--stdout",
                "mga-baseline.stdout.log",
                "--stderr",
                "mga-baseline.stderr.log",
                "--elapsed-milliseconds",
                "10",
                "--argument=--scale",
                "--argument=development",
                "--argument=--output",
                "--argument=mga-baseline.result.tmp.json",
            ]
        )
        self.assertEqual(
            [
                "--scale",
                "development",
                "--output",
                "mga-baseline.result.tmp.json",
            ],
            parsed.argument,
        )


class MetamorphicReceiptContractTests(unittest.TestCase):
    COMMIT = "1" * 40

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        self.cell_dir = self.root / "cells"
        self.plan = self.root / "cell-plan.json"
        cells.atomic_json(self.plan, cells.plan_document())
        self.binaries: dict[str, Path] = {}
        for family in cells.FAMILIES:
            path = self.root / f"{family['example']}.exe"
            path.write_bytes(f"fake-{family['family']}-producer".encode())
            self.binaries[family["family"]] = path.resolve()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def bindings(self) -> list[str]:
        return [f"{family}={path}" for family, path in self.binaries.items()]

    def seal_all(self) -> None:
        for spec in cells.expected_cells():
            paths = cells.paths_for(self.cell_dir, spec["cell_id"])
            paths["temporary"].parent.mkdir(parents=True, exist_ok=True)
            paths["stdout"].parent.mkdir(parents=True, exist_ok=True)
            cells.atomic_json(
                paths["temporary"],
                {
                    spec["identity_field"]: spec["producer_identity"],
                    "scale": "development",
                    "metamorphism": spec["axis"],
                    "workers": spec["workers"],
                },
            )
            paths["stdout"].write_text("ok\n", encoding="utf-8")
            paths["stderr"].write_text("", encoding="utf-8")
            executable = self.binaries[spec["family"]]
            cells.seal_cell(
                SimpleNamespace(
                    plan=self.plan,
                    cell_dir=self.cell_dir,
                    cell_id=spec["cell_id"],
                    source_commit=self.COMMIT,
                    executable=executable,
                    executable_sha256=cells.sha256_file(executable),
                    temporary_result=paths["temporary"],
                    stdout=paths["stdout"],
                    stderr=paths["stderr"],
                    elapsed_milliseconds=10,
                    argument=cells.expected_arguments(self.cell_dir, spec["cell_id"]),
                )
            )

    def test_all_cells_resume_and_aggregate_from_hash_bound_receipts(self) -> None:
        self.seal_all()
        status = cells.status_document(
            SimpleNamespace(
                plan=self.plan,
                cell_dir=self.cell_dir,
                source_commit=self.COMMIT,
                binary=self.bindings(),
            )
        )
        self.assertTrue(status["complete"])
        self.assertEqual(25, len(status["valid_cell_ids"]))
        self.assertEqual([], status["invalid_cells"])

        report = self.root / "report.json"
        cells.atomic_json(
            report,
            {"report_id": cells.REPORT_ID, "status": "passed"},
        )
        build = self.root / "build.json"
        build_stdout = self.root / "build.stdout.log"
        build_stderr = self.root / "build.stderr.log"
        build_stdout.write_text("built\n", encoding="utf-8")
        build_stderr.write_text("", encoding="utf-8")
        build_command = cells.expected_build_command()
        cells.atomic_json(
            build,
            {
                "receipt_id": "qpls.multimod.metamorphic.single-build-receipt.v1",
                "status": "passed",
                "source_commit": self.COMMIT,
                "cargo_invocation_count": 1,
                "command": build_command,
                "command_sha256": cells.sha256_json(build_command),
                "exit_code": 0,
                "elapsed_milliseconds": 10,
                "stdout_path": str(build_stdout),
                "stdout_sha256": cells.sha256_file(build_stdout),
                "stdout_size": build_stdout.stat().st_size,
                "stderr_path": str(build_stderr),
                "stderr_sha256": cells.sha256_file(build_stderr),
                "stderr_size": build_stderr.stat().st_size,
                "binary_sha256": {
                    family: cells.sha256_file(path)
                    for family, path in sorted(self.binaries.items())
                },
            },
        )
        execution = self.root / "execution.json"
        cells.aggregate(
            SimpleNamespace(
                plan=self.plan,
                cell_dir=self.cell_dir,
                source_commit=self.COMMIT,
                binary=self.bindings(),
                build_receipt=build,
                report=report,
                output=execution,
            )
        )
        receipt = json.loads(execution.read_text(encoding="utf-8"))
        self.assertEqual(cells.EXECUTION_RECEIPT_ID, receipt["receipt_id"])
        self.assertEqual(25, receipt["cell_count"])
        self.assertEqual(4, len(receipt["baseline_root_cell_ids"]))
        self.assertTrue(receipt["one_cargo_build"])
        self.assertTrue(receipt["verifier_unchanged"])

    def test_tampered_baseline_invalidates_its_dependent_axes_only(self) -> None:
        self.seal_all()
        baseline = cells.paths_for(self.cell_dir, "mga-baseline")["result"]
        baseline.write_text("{}\n", encoding="utf-8")
        status = cells.status_document(
            SimpleNamespace(
                plan=self.plan,
                cell_dir=self.cell_dir,
                source_commit=self.COMMIT,
                binary=self.bindings(),
            )
        )
        invalid = {row["cell_id"] for row in status["invalid_cells"]}
        self.assertEqual(
            {
                "mga-baseline",
                "mga-seed_repeat",
                "mga-row_reverse",
                "mga-input_column_reverse",
                "mga-declaration_reverse",
                "mga-worker_parallel",
            },
            invalid,
        )
        self.assertEqual(19, len(status["valid_cell_ids"]))


class MetamorphicWrapperStaticTests(unittest.TestCase):
    def test_wrapper_has_one_build_direct_cells_and_hard_caps(self) -> None:
        wrapper = (HERE / "run_multimod_metamorphic_qualification_v1.ps1").read_text(
            encoding="utf-8"
        )
        lowered = wrapper.lower()
        self.assertNotIn("cargo run", lowered)
        self.assertIn('"build", "--quiet", "--locked"', wrapper)
        for family in cells.FAMILIES:
            self.assertEqual(2, wrapper.count(family["example"]))
        self.assertIn("[ValidateRange(1, 4)]", wrapper)
        self.assertIn("[ValidateRange(60, 1800)]", wrapper)
        self.assertIn("[ValidateRange(600, 6480)]", wrapper)
        self.assertIn("[ValidateRange(600, 6600)]", wrapper)
        self.assertIn("$Job.Process.Kill($true)", wrapper)
        self.assertIn("[System.Diagnostics.ProcessStartInfo]::new()", wrapper)
        self.assertLess(
            wrapper.index('Invoke-CellPhase -Name "baseline-root"'),
            wrapper.index('Invoke-CellPhase -Name "dependent-axis"'),
        )
        self.assertIn("verify_multimod_metamorphic_qualification_v1.py", wrapper)
        self.assertIn("multimod_metamorphic_cells_v1.py", wrapper)


if __name__ == "__main__":
    unittest.main()
