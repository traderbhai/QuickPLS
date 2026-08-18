#!/usr/bin/env python3
"""Unit tests for the CLI-independent current-product PLS work seam."""

from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

import pls_algorithm_v1_current_product_work as work  # noqa: E402
import pls_algorithm_v1_qualification_factory as factory  # noqa: E402
from qualification_spec_v2 import strict_load_json  # noqa: E402


class PlsAlgorithmV1CurrentProductWorkTests(unittest.TestCase):
    @staticmethod
    def _request_model() -> dict[str, object]:
        return {
            "model": {
                "variables": [
                    {"kind": "composite", "id": "stable-alpha", "label": "X"},
                    {"kind": "composite", "id": "stable-beta", "label": "Y"},
                    {
                        "kind": "observed",
                        "id": "stable-observed-one",
                        "source_column": "x1",
                    },
                    {
                        "kind": "observed",
                        "id": "stable-observed-two",
                        "source_column": "x2",
                    },
                    {
                        "kind": "observed",
                        "id": "stable-observed-three",
                        "source_column": "y1",
                    },
                    {
                        "kind": "observed",
                        "id": "stable-observed-four",
                        "source_column": "y2",
                    },
                ],
                "relations": [
                    {
                        "kind": "measurement_effect",
                        "construct": "stable-alpha",
                        "indicator": "stable-observed-one",
                    },
                    {
                        "kind": "measurement_effect",
                        "construct": "stable-alpha",
                        "indicator": "stable-observed-two",
                    },
                    {
                        "kind": "measurement_effect",
                        "construct": "stable-beta",
                        "indicator": "stable-observed-three",
                    },
                    {
                        "kind": "measurement_effect",
                        "construct": "stable-beta",
                        "indicator": "stable-observed-four",
                    },
                    {
                        "kind": "structural",
                        "source": "stable-alpha",
                        "target": "stable-beta",
                    },
                ],
            }
        }

    @staticmethod
    def _harness_report(executable: Path) -> dict[str, object]:
        executable_sha256 = factory.sha256_file(executable)
        return {
            "schema_version": 1,
            "report_kind": "pls_algorithm_v1_current_product_harness_v1",
            "work_evidence_only": True,
            "qualification_ready": False,
            "promotion_authority": False,
            "customer_cli_invoked": False,
            "customer_registry_admission_invoked": False,
            "candidate_receipt_descriptors": [],
            "compiled_identity": {
                "source_set_sha256": "1" * 64,
                "scenario_set_sha256": "2" * 64,
                "executable": {
                    "path": str(executable.resolve()),
                    "sha256": executable_sha256,
                    "size_bytes": executable.stat().st_size,
                },
            },
            "capability_cell": {
                "registry_schema_version": 2,
                "capability_id": factory.CAPABILITY_ID,
                "cell_id": factory.CELL_ID,
                "capability_version": factory.METHOD_VERSION,
            },
            "archive": {
                "schema_version": 6,
                "canonical_result_document_count": len(work.VARIANTS),
                "resident_dataset_verified": True,
                "models_and_recipes_verified": True,
                "atomic_append_verified": True,
                "final_reopen_verified": True,
            },
            "variants": [
                {
                    "variant": label,
                    "recipe_v4_compiled": True,
                    "real_runner_executed": True,
                    "product_canonical_builder_executed": True,
                    "schema6_append_reopened": True,
                    "method_version": factory.METHOD_VERSION,
                    "request": {
                        "path": f"{label}.request.json",
                        "size_bytes": 1,
                        "sha256": "4" * 64,
                    },
                    "execution": {
                        "path": f"{label}.execution.json",
                        "size_bytes": 1,
                        "sha256": "3" * 64,
                    },
                }
                for label in work.VARIANTS
            ],
        }

    def test_commands_build_only_ignored_desktop_harness_and_never_customer_cli(self) -> None:
        build = work._cargo_build_command("cargo")  # noqa: SLF001
        self.assertEqual(build[:4], ["cargo", "test", "-p", "quickpls-desktop"])
        self.assertIn("--no-run", build)
        executable = Path("target/debug/deps/quickpls_desktop_lib-fixture.exe")
        run = work._harness_command(executable)  # noqa: SLF001
        self.assertEqual(run[0], str(executable))
        self.assertIn("--ignored", run)
        self.assertIn("--exact", run)
        flattened = " ".join([*build, *run]).lower()
        self.assertNotIn("qpls run", flattened)
        self.assertNotIn("allow-experimental", flattened)
        self.assertNotIn("qpls-cli", flattened)

    def test_product_seam_source_contract_is_exact_and_binds_phase_two(self) -> None:
        self.assertEqual(
            set(work.REQUIRED_PRODUCT_SEAM_SOURCE_PATHS),
            set(factory.PRODUCT_BUILD_SOURCE_PATHS),
        )
        self.assertEqual(
            len(work.REQUIRED_PRODUCT_SEAM_SOURCE_PATHS),
            len(set(work.REQUIRED_PRODUCT_SEAM_SOURCE_PATHS)),
        )
        self.assertTrue(
            set(work.REQUIRED_PRODUCT_SEAM_SOURCE_PATHS).issubset(factory.SOURCE_PATHS)
        )
        self.assertTrue(
            {
                "crates/qpls-core/src/analysis_recipe_v4.rs",
                "crates/qpls-core/src/compiled_pls_plan_v2.rs",
                "crates/qpls-core/src/recipe_v4_compiler.rs",
                "crates/qpls-project/src/pls_score_execution_v2.rs",
                "crates/qpls-project/src/project_schema_v6.rs",
                "crates/qpls-runner/src/recipe_v4_pls_execution.rs",
                "src-tauri/src/pls_algorithm_current_product_qualification.rs",
                "src-tauri/src/project_schema6_result_read.rs",
                "src-tauri/src/recipe_v4_canonical_result.rs",
            }.issubset(work.REQUIRED_PRODUCT_SEAM_SOURCE_PATHS)
        )
        self.assertFalse(
            any("qpls-cli" in path for path in work.REQUIRED_PRODUCT_SEAM_SOURCE_PATHS)
        )

    def test_cargo_json_selects_exactly_one_desktop_lib_test_executable(self) -> None:
        expected = Path("target/debug/deps/quickpls_desktop_lib-fixture.exe").resolve()
        rows = [
            {
                "reason": "compiler-artifact",
                "target": {"name": "qpls_core"},
                "profile": {"test": True},
                "executable": "target/debug/deps/qpls_core-fixture.exe",
            },
            {
                "reason": "compiler-artifact",
                "target": {"name": work.HARNESS_TARGET},
                "profile": {"test": True},
                "executable": str(expected),
            },
        ]
        stdout = "\n".join(json.dumps(row) for row in rows)
        self.assertEqual(work._extract_harness_executable(stdout), expected)  # noqa: SLF001
        with self.assertRaisesRegex(ValueError, "exactly one"):
            work._extract_harness_executable("")  # noqa: SLF001
        duplicate = stdout + "\n" + json.dumps(
            {
                "reason": "compiler-artifact",
                "target": {"name": work.HARNESS_TARGET},
                "profile": {"test": True},
                "executable": str(expected.with_name("other.exe")),
            }
        )
        with self.assertRaisesRegex(ValueError, "exactly one"):
            work._extract_harness_executable(duplicate)  # noqa: SLF001

    def test_registry_contract_requires_exact_scoped_standard_state(self) -> None:
        registry = strict_load_json(factory.REGISTRY_PATH)
        exact = work._assert_registry_contract(registry)  # noqa: SLF001
        self.assertEqual(exact["coverage_state"], "partial")
        self.assertEqual(exact["evidence_state"], "release_qualified")
        self.assertEqual(exact["surface"], "standard")
        promoted = copy.deepcopy(registry)
        row = next(
            item
            for item in promoted["capabilities"]
            if item["capability_id"] == factory.CAPABILITY_ID
        )
        cell = next(
            item for item in row["option_cells"] if item["cell_id"] == factory.CELL_ID
        )
        cell["evidence_state"] = "engine_only"
        with self.assertRaisesRegex(ValueError, "scoped-Standard"):
            work._assert_registry_contract(promoted)  # noqa: SLF001

    def test_product_value_reader_accepts_real_recipe_v4_execution_shape(self) -> None:
        result = {
            "schema_version": 1,
            "provenance": {"adapter_version": "fixture"},
            "estimation": {
                "paths": [{"source": "x", "target": "y", "coefficient": 0.5}],
                "outer_estimates": [
                    {
                        "construct": "x",
                        "indicator": "x1",
                        "weight": 0.7,
                        "loading": 0.8,
                    }
                ],
            },
        }
        values = work._product_values(result)  # noqa: SLF001
        self.assertEqual(values[("path", "x", "y", "")], 0.5)
        self.assertEqual(values[("weight", "x", "", "x1")], 0.7)
        self.assertEqual(values[("loading", "x", "", "x1")], 0.8)

    def test_oracle_values_are_rekeyed_from_exact_model_membership_not_prefixes(self) -> None:
        identity_map = work._derive_stable_construct_identity_map(  # noqa: SLF001
            self._request_model(),
            work.ORACLE_CONSTRUCT_BLOCKS,
            work.ORACLE_STRUCTURAL_PATHS,
        )
        self.assertEqual(
            identity_map,
            {"x": "stable-alpha", "y": "stable-beta"},
        )
        causal = copy.deepcopy(self._request_model())
        for relation in causal["model"]["relations"]:  # type: ignore[index]
            if relation["kind"] == "measurement_effect":
                relation["kind"] = "measurement_causal"
                relation["composite"] = relation.pop("construct")
        self.assertEqual(
            work._derive_stable_construct_identity_map(  # noqa: SLF001
                causal,
                work.ORACLE_CONSTRUCT_BLOCKS,
                work.ORACLE_STRUCTURAL_PATHS,
            ),
            identity_map,
        )
        logical = {
            ("path", "x", "y", ""): 0.5,
            ("loading", "x", "", "x1"): 0.8,
            ("weight", "x", "", "x1"): 0.7,
        }
        product = {
            ("path", "stable-alpha", "stable-beta", ""): 0.5,
            ("loading", "stable-alpha", "", "x1"): 0.8,
            ("weight", "stable-alpha", "", "x1"): 0.7,
        }
        raw = work._comparison("namespace_mismatch", product, logical)  # noqa: SLF001
        self.assertFalse(raw["exact_key_membership"])
        stable = work._rekey_oracle_construct_identities(  # noqa: SLF001
            logical, identity_map
        )
        self.assertEqual(stable, product)
        self.assertTrue(
            work._comparison("stable_namespace", product, stable)["passed"]  # noqa: SLF001
        )

    def test_stable_identity_mapping_rejects_ambiguous_missing_and_non_bijective_models(
        self,
    ) -> None:
        ambiguous = copy.deepcopy(self._request_model())
        ambiguous["model"]["variables"].append(  # type: ignore[index]
            {"kind": "composite", "id": "stable-alpha-copy"}
        )
        ambiguous["model"]["relations"].extend(  # type: ignore[index]
            [
                {
                    "kind": "measurement_effect",
                    "construct": "stable-alpha-copy",
                    "indicator": "stable-observed-one",
                },
                {
                    "kind": "measurement_effect",
                    "construct": "stable-alpha-copy",
                    "indicator": "stable-observed-two",
                },
            ]
        )
        with self.assertRaisesRegex(ValueError, "ambiguous"):
            work._derive_stable_construct_identity_map(  # noqa: SLF001
                ambiguous,
                work.ORACLE_CONSTRUCT_BLOCKS,
                work.ORACLE_STRUCTURAL_PATHS,
            )

        missing = copy.deepcopy(self._request_model())
        missing["model"]["variables"][3]["source_column"] = "not-x2"  # type: ignore[index]
        with self.assertRaisesRegex(ValueError, "missing"):
            work._derive_stable_construct_identity_map(  # noqa: SLF001
                missing,
                work.ORACLE_CONSTRUCT_BLOCKS,
                work.ORACLE_STRUCTURAL_PATHS,
            )

        with self.assertRaisesRegex(ValueError, "non-bijective"):
            work._derive_stable_construct_identity_map(  # noqa: SLF001
                self._request_model(),
                {
                    "x": ("x1", "x2"),
                    "x_alias": ("x1", "x2"),
                    "y": ("y1", "y2"),
                },
                work.ORACLE_STRUCTURAL_PATHS,
            )

    def test_failed_run_diagnostics_are_non_promotional_and_create_new(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            build_root = Path(directory)
            temporary = build_root / "temporary"
            temporary.mkdir()
            diagnostics = {
                "schema_version": 1,
                "report_kind": (
                    "pls_algorithm_v1_current_product_failure_diagnostics_v1"
                ),
                "passed": False,
                "work_evidence_only": True,
                "qualification_ready": False,
                "promotion_authority": False,
                "candidate_receipt_descriptors": [],
                "customer_cli_invoked": False,
                "customer_registry_admission_invoked": False,
            }
            destination = work._preserve_failed_run(  # noqa: SLF001
                temporary=temporary,
                build_root=build_root,
                executable_descriptor={"sha256": "a" * 64},
                diagnostics=diagnostics,
            )
            self.assertTrue(temporary.is_dir())
            self.assertEqual(
                strict_load_json(destination / "failed_current_product_diagnostics.json"),
                diagnostics,
            )

            second = build_root / "second-temporary"
            second.mkdir()
            with self.assertRaisesRegex(FileExistsError, "already exist"):
                work._preserve_failed_run(  # noqa: SLF001
                    temporary=second,
                    build_root=build_root,
                    executable_descriptor={"sha256": "a" * 64},
                    diagnostics=diagnostics,
                )
            promotional = dict(diagnostics, qualification_ready=True)
            with self.assertRaisesRegex(ValueError, "promotional or incomplete"):
                work._preserve_failed_run(  # noqa: SLF001
                    temporary=second,
                    build_root=build_root,
                    executable_descriptor={"sha256": "b" * 64},
                    diagnostics=promotional,
                )

    def test_artifact_resolver_rejects_digest_mismatch_and_escape(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "execution.json"
            artifact.write_text("{}\n", encoding="utf-8")
            descriptor = {
                "path": artifact.name,
                "size_bytes": artifact.stat().st_size,
                "sha256": factory.sha256_file(artifact),
            }
            self.assertEqual(
                work._resolve_artifact(root, descriptor),  # noqa: SLF001
                artifact.resolve(),
            )
            drifted = dict(descriptor, sha256="0" * 64)
            with self.assertRaisesRegex(ValueError, "digest mismatch"):
                work._resolve_artifact(root, drifted)  # noqa: SLF001
            escaped = dict(descriptor, path="../outside.json")
            with self.assertRaises((FileNotFoundError, ValueError)):
                work._resolve_artifact(root, escaped)  # noqa: SLF001

    def test_harness_report_validation_fails_closed_on_promotional_or_identity_drift(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "quickpls_desktop_lib-test.exe"
            executable.write_bytes(b"test executable")
            executable_descriptor = {
                "sha256": factory.sha256_file(executable),
                "size_bytes": executable.stat().st_size,
            }
            report = self._harness_report(executable)
            variants = work._validate_harness_report(  # noqa: SLF001
                report,
                executable=executable,
                executable_descriptor=executable_descriptor,
                source_set_sha256="1" * 64,
                scenario_set_sha256="2" * 64,
            )
            self.assertEqual(
                [row["variant"] for row in variants],
                list(work.VARIANTS),
            )

            promotional = copy.deepcopy(report)
            promotional["qualification_ready"] = True
            with self.assertRaisesRegex(ValueError, "promotional or drifted"):
                work._validate_harness_report(  # noqa: SLF001
                    promotional,
                    executable=executable,
                    executable_descriptor=executable_descriptor,
                    source_set_sha256="1" * 64,
                    scenario_set_sha256="2" * 64,
                )

            receipts = copy.deepcopy(report)
            receipts["candidate_receipt_descriptors"] = [{"role": "kernel_execution"}]
            with self.assertRaisesRegex(ValueError, "must not emit"):
                work._validate_harness_report(  # noqa: SLF001
                    receipts,
                    executable=executable,
                    executable_descriptor=executable_descriptor,
                    source_set_sha256="1" * 64,
                    scenario_set_sha256="2" * 64,
                )

            substituted = copy.deepcopy(report)
            substituted["compiled_identity"]["executable"]["sha256"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "self-executable identity mismatch"):
                work._validate_harness_report(  # noqa: SLF001
                    substituted,
                    executable=executable,
                    executable_descriptor=executable_descriptor,
                    source_set_sha256="1" * 64,
                    scenario_set_sha256="2" * 64,
                )

            incomplete = copy.deepcopy(report)
            incomplete["variants"].pop()
            with self.assertRaisesRegex(ValueError, "variant set is not exact"):
                work._validate_harness_report(  # noqa: SLF001
                    incomplete,
                    executable=executable,
                    executable_descriptor=executable_descriptor,
                    source_set_sha256="1" * 64,
                    scenario_set_sha256="2" * 64,
                )


if __name__ == "__main__":
    unittest.main()
