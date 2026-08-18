#!/usr/bin/env python3
"""Focused mutation/contract tests for the PLS power v1 evidence lane."""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
import zipfile
from copy import deepcopy
from pathlib import Path

from pls_sample_size_power_export_gate import _normalize_rows
from pls_sample_size_power_persistence_gate import write_project_archive
from pls_sample_size_power_persistence_gate import ARCHIVE_CLAIM_BOUNDARY
from pls_sample_size_power_method_promotion_audit import (
    archive_claim_boundary_contract,
)
from pls_sample_size_power_simulation import (
    FEATURE_ID,
    METHOD_VERSION,
    ROOT,
    canonical_recipe,
    compare_independent_power,
    manifest,
    scientific_recipe_from_canonical,
    validate_product_analysis,
    wilson_interval,
)


def _analysis(recipe: dict, first_rejections: int, second_rejections: int) -> dict:
    requested = recipe["method_config"]["monte_carlo_replicates"]
    rows = []
    outcomes = []
    for sample_size, rejections in zip(
        recipe["method_config"]["sample_size_grid"],
        (first_rejections, second_rejections),
    ):
        lower, upper = wilson_interval(
            rejections,
            requested,
            recipe["method_config"]["interval_confidence_level"],
        )
        rows.append(
            {
                "sample_size": sample_size,
                "requested_replicates": requested,
                "attempted_replicates": requested,
                "successful_replicates": requested,
                "failed_replicates": 0,
                "rejections": rejections,
                "achieved_power": rejections / requested,
                "confidence_lower": lower,
                "confidence_upper": upper,
                "qualifies": lower >= recipe["method_config"]["target_power"],
            }
        )
        for replicate in range(requested):
            rejected = replicate < rejections
            outcomes.append(
                {
                    "sample_size": sample_size,
                    "replicate_index": replicate,
                    "stream_identity": f"{sample_size}-{replicate}",
                    "attempted": True,
                    "successful": True,
                    "converged": True,
                    "target_estimate": 0.4,
                    "p_value_two_sided": 0.01 if rejected else 0.2,
                    "rejected": rejected,
                    "failure_code": None,
                    "failure_message": None,
                }
            )
    selected = next((row["sample_size"] for row in rows if row["qualifies"]), None)
    return {
        "capability_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "rows": rows,
        "outcomes": outcomes,
        "decision": (
            {"status": "reached", "sample_size": selected}
            if selected is not None
            else {"status": "not_reached"}
        ),
        "workload": {"planned_datasets": requested * 2},
    }


class PlsSampleSizePowerV1Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.recipe = canonical_recipe(
            "v2:" + "a" * 64,
            name="focused_unit",
            population_path=0.4,
            sample_size_grid=(60, 120),
            monte_carlo_replicates=100,
            bootstrap_replicates=99,
            target_power=0.50,
            seed=123,
            workers=4,
        )

    def test_product_recipe_projects_exactly_to_independent_contract(self) -> None:
        scientific = scientific_recipe_from_canonical(self.recipe)
        self.assertEqual(scientific["capability_id"], FEATURE_ID)
        self.assertEqual(scientific["method_version"], METHOD_VERSION)
        self.assertEqual(scientific["inference"], "case_bootstrap_normal_reference_two_sided")
        self.assertEqual(scientific["sample_size_grid"], [60, 120])
        self.assertEqual(scientific["master_seed"], 123)
        self.assertEqual(scientific["workers"], 4)
        self.assertNotIn("dataset_fingerprint", scientific)

    def test_recomputation_rejects_changed_denominator_and_decision(self) -> None:
        analysis = _analysis(self.recipe, 60, 80)
        valid = validate_product_analysis(self.recipe, analysis)
        self.assertTrue(valid["passed"], valid)

        changed_denominator = deepcopy(analysis)
        changed_denominator["rows"][0]["requested_replicates"] = 99
        self.assertFalse(validate_product_analysis(self.recipe, changed_denominator)["passed"])

        changed_decision = deepcopy(analysis)
        changed_decision["decision"] = {"status": "not_reached"}
        self.assertFalse(validate_product_analysis(self.recipe, changed_decision)["passed"])

    def test_failed_replicate_must_be_named_and_remain_non_rejection(self) -> None:
        analysis = _analysis(self.recipe, 60, 80)
        outcome = analysis["outcomes"][0]
        outcome.update(
            {
                "successful": False,
                "converged": False,
                "target_estimate": None,
                "p_value_two_sided": None,
                "rejected": False,
                "failure_code": "fit_failed",
                "failure_message": "named failure",
            }
        )
        analysis["rows"][0]["successful_replicates"] -= 1
        analysis["rows"][0]["failed_replicates"] += 1
        analysis["rows"][0]["rejections"] -= 1
        lower, upper = wilson_interval(59, 100, 0.95)
        analysis["rows"][0].update(
            {
                "achieved_power": 0.59,
                "confidence_lower": lower,
                "confidence_upper": upper,
                "qualifies": lower >= 0.50,
            }
        )
        analysis["decision"] = {"status": "reached", "sample_size": 120}
        self.assertTrue(validate_product_analysis(self.recipe, analysis)["passed"])

        analysis["outcomes"][0]["failure_code"] = ""
        self.assertFalse(validate_product_analysis(self.recipe, analysis)["passed"])

    def test_independent_comparison_requires_target_lower_bound_at_selected_n(self) -> None:
        product = _analysis(self.recipe, 60, 80)
        reference = deepcopy(product)
        comparison = compare_independent_power(product, reference, target_power=0.50)
        self.assertTrue(comparison["passed"], comparison)

        reference["rows"][0]["confidence_lower"] = 0.49
        comparison = compare_independent_power(product, reference, target_power=0.50)
        self.assertFalse(comparison["passed"])

    def test_project_archive_rewriter_recomputes_all_non_manifest_checksums(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base = root / "base.qpls"
            manifest_document = {
                "schema_version": 5,
                "checksum_algorithm": "sha256",
                "checksums": {
                    "data/a.arrow": hashlib.sha256(b"arrow").hexdigest(),
                    "project.json": hashlib.sha256(b"{}\n").hexdigest(),
                },
            }
            with zipfile.ZipFile(base, "w") as archive:
                archive.writestr("data/a.arrow", b"arrow")
                archive.writestr("project.json", b"{}\n")
                archive.writestr("manifest.json", json.dumps(manifest_document))
            output = root / "rewritten.qpls"
            project = {"recipes": [self.recipe], "results": []}
            write_project_archive(base, output, project_document=project)
            with zipfile.ZipFile(output) as archive:
                updated = json.loads(archive.read("manifest.json"))
                self.assertEqual(
                    updated["checksums"]["project.json"],
                    hashlib.sha256(archive.read("project.json")).hexdigest(),
                )
                self.assertEqual(
                    updated["checksums"]["data/a.arrow"],
                    hashlib.sha256(b"arrow").hexdigest(),
                )

    def test_archive_claim_boundary_is_explicitly_structural_not_semantic(self) -> None:
        self.assertFalse(ARCHIVE_CLAIM_BOUNDARY["semantic_replay_performed"])
        self.assertFalse(ARCHIVE_CLAIM_BOUNDARY["coordinated_rewrite_authenticated"])
        self.assertIn("stored replicate ledger", ARCHIVE_CLAIM_BOUNDARY["verified"])
        limitation = ARCHIVE_CLAIM_BOUNDARY["not_verified"]
        for fragment in (
            "PLS estimates",
            "bootstrap fits",
            "target estimates",
            "p values",
            "rejection flags",
            "outcome digest",
            "derived power rows",
            "grid decision",
            "outer archive checksum",
        ):
            self.assertIn(fragment, limitation)
        contract = archive_claim_boundary_contract()
        self.assertTrue(contract["passed"], contract)

    def test_smoke_profile_and_packaged_wrapper_are_structurally_non_claiming(self) -> None:
        simulation = (ROOT / "validation/pls_sample_size_power_simulation.py").read_text(
            encoding="utf-8"
        )
        wrapper = (
            ROOT / "validation/run_pls_sample_size_power_native_acceptance.ps1"
        ).read_text(encoding="utf-8")
        self.assertIn('"qualification_profile_complete": False', simulation)
        self.assertIn('"promotion_evidence_written": False', simulation)
        self.assertNotIn("write_identity_report(\n            \"simulation_report\"", simulation.split("def smoke_profile", 1)[1].split("def main", 1)[0])
        self.assertNotIn("Start-Process", wrapper)
        self.assertIn("No release evidence was written", wrapper)
        self.assertGreaterEqual(wrapper.count("throw "), 4)

    def test_method_spec_and_boundary_gate_share_reachable_contract_fragments(self) -> None:
        document = (ROOT / "docs/methods/PLS_SAMPLE_SIZE_POWER_V1.md").read_text(
            encoding="utf-8"
        )
        audit = (
            ROOT / "validation/pls_sample_size_power_method_promotion_audit.py"
        ).read_text(encoding="utf-8")
        boundary = (
            ROOT / "validation/pls_sample_size_power_boundary_gate.py"
        ).read_text(encoding="utf-8")
        inference_fragment = "case-bootstrap normal-reference"
        boundary_fragment = "declared loadings must map one-to-one to model indicators"
        self.assertIn(inference_fragment, document)
        self.assertIn(f'"{inference_fragment}"', audit)
        self.assertIn(f'expected_fragment="{boundary_fragment}"', boundary)

    def test_manifest_sources_are_exactly_method_scoped_and_release_is_not_fabricated(self) -> None:
        document = manifest()
        sources = document["qualification"]["source_requirements"]
        self.assertIn("validation/pls_sample_size_power_simulation.py", sources["simulation_report"])
        self.assertIn("validation/pls_sample_size_power_boundary_gate.py", sources["boundary_report"])
        self.assertIn("validation/pls_sample_size_power_persistence_gate.py", sources["persistence_report"])
        self.assertIn("validation/pls_sample_size_power_export_gate.py", sources["export_report"])
        self.assertIn("validation/pls_sample_size_power_method_promotion_audit.py", sources["method_audit"])
        self.assertIn(
            "validation/run_pls_sample_size_power_native_acceptance.ps1",
            sources["packaged_acceptance"],
        )
        self.assertEqual(document["qualification"]["evidence"]["release_qualified"], [])

    def test_csv_xlsx_row_normalization_drops_only_trailing_empty_cells(self) -> None:
        rows = [["a", "", ""], ["", "b", ""]]
        self.assertEqual(_normalize_rows(rows), [["a"], ["", "b"]])


if __name__ == "__main__":
    unittest.main()
