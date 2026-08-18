#!/usr/bin/env python3
"""Mutation-focused tests for the fail-closed Phase-5 CB-SEM audit."""

from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent

import sys

sys.path.insert(0, str(VALIDATION))

from phase5_cbsem_expansion_audit import (  # noqa: E402
    COMMON_SOURCES,
    CONTRACT_PATH,
    DuplicateKeyError,
    EXPECTED_TRACK_IDS,
    audit,
    evaluate_track,
    strict_load,
    validate_contract,
)


class Phase5CbsemExpansionAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = strict_load(CONTRACT_PATH)

    def test_contract_inventory_dependencies_and_completion_rule_are_closed(self) -> None:
        self.assertEqual(validate_contract(self.contract), [])
        self.assertEqual(
            tuple(track["id"] for track in self.contract["tracks"]),
            EXPECTED_TRACK_IDS,
        )
        self.assertEqual(
            tuple(self.contract["completion_rule"]["required_tracks"]),
            EXPECTED_TRACK_IDS,
        )
        self.assertTrue(
            self.contract["completion_rule"]["preview_relabeling_forbidden"]
        )
        bootstrap = next(
            track for track in self.contract["tracks"] if track["id"] == "cbsem_bootstrap"
        )
        self.assertTrue(bootstrap["candidate_implementation"])
        self.assertEqual(bootstrap["current_state"], "absent")
        self.assertTrue(
            any(
                "does not replay every ML fit" in blocker
                for blocker in bootstrap["blockers"]
            )
        )

    def test_contract_rejects_inventory_drift_and_dependency_cycles(self) -> None:
        missing = copy.deepcopy(self.contract)
        missing["tracks"].pop()
        missing["completion_rule"]["required_tracks"].pop()
        self.assertTrue(validate_contract(missing))

        cycle = copy.deepcopy(self.contract)
        cycle["tracks"][0]["dependencies"] = ["cbsem_bootstrap"]
        cycle_errors = validate_contract(cycle)
        self.assertTrue(
            any("dependency cycle" in error for error in cycle_errors),
            cycle_errors,
        )

    def test_strict_json_rejects_duplicate_keys(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "duplicate.json"
            path.write_text('{"passed":true,"passed":false}\n', encoding="utf-8")
            with self.assertRaises(DuplicateKeyError):
                strict_load(path)

    def test_forbidden_engine_marker_fails_closed(self) -> None:
        track = copy.deepcopy(
            next(item for item in self.contract["tracks"] if item["id"] == "cbsem_fiml")
        )
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for relative in COMMON_SOURCES:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(f"fixture {relative}\n", encoding="utf-8")

            contract_source = root / "crates/qpls-core/src/contract.rs"
            contract_source.parent.mkdir(parents=True, exist_ok=True)
            contract_source.write_text(
                "pub enum MissingDataPolicy { ListwiseDeletion }\n",
                encoding="utf-8",
            )
            validation_source = root / "crates/qpls-core/src/validation.rs"
            validation_source.parent.mkdir(parents=True, exist_ok=True)
            validation_source.write_text("cbsem.listwise_required\n", encoding="utf-8")
            estimation_source = root / "crates/qpls-estimation/src/pls.rs"
            estimation_source.parent.mkdir(parents=True, exist_ok=True)
            estimation_source.write_text("bounded ordinary ML fixture\n", encoding="utf-8")

            clean = evaluate_track(track, root=root, contract_errors=[])
            self.assertTrue(clean["passed"], clean)
            self.assertEqual(clean["derived_current_state"], "absent")

            untracked_manifest = root / "validation/methods/untracked.manifest.json"
            untracked_manifest.parent.mkdir(parents=True, exist_ok=True)
            untracked_manifest.write_text(
                '{"feature":{"id":"qpls3.cbsem.fiml"}}\n',
                encoding="utf-8",
            )
            untracked = evaluate_track(track, root=root, contract_errors=[])
            self.assertFalse(untracked["passed"])
            self.assertFalse(untracked["checks"]["untracked_promotion_manifests_absent"])
            self.assertEqual(
                untracked["unexpected_promotion_manifests"],
                ["validation/methods/untracked.manifest.json"],
            )
            untracked_manifest.unlink()

            contract_source.write_text(
                "pub enum MissingDataPolicy { ListwiseDeletion }\n"
                "let unsupported_promotion = MissingDataPolicy::Fiml;\n",
                encoding="utf-8",
            )
            mutated = evaluate_track(track, root=root, contract_errors=[])
            self.assertFalse(mutated["passed"])
            self.assertFalse(mutated["checks"]["forbidden_engine_markers_absent"])
            marker = next(
                row
                for row in mutated["forbidden_engine_markers"]
                if row["marker"] == "MissingDataPolicy::Fiml"
            )
            self.assertEqual(marker["locations"], ["crates/qpls-core/src/contract.rs"])

    def test_current_repository_boundary_is_truthful_but_phase_is_incomplete(self) -> None:
        report = audit()
        self.assertTrue(report["passed"], report)
        self.assertFalse(report["phase5_complete"])
        self.assertFalse(report["competitor_claim_for_expanded_cbsem_admissible"])
        self.assertEqual(report["required_track_count"], len(EXPECTED_TRACK_IDS))
        states = {
            row["track_id"]: row["derived_current_state"]
            for row in report["tracks"]
        }
        self.assertEqual(states["cbsem_ml_baseline"], "release_qualified")
        self.assertTrue(
            all(
                state == "absent"
                for track_id, state in states.items()
                if track_id != "cbsem_ml_baseline"
            ),
            states,
        )
        self.assertEqual(
            report["state_counts"],
            {"absent": 9, "release_qualified": 1},
        )
        self.assertEqual(report["release_qualified_track_count"], 1)
        baseline_report = next(
            row for row in report["tracks"] if row["track_id"] == "cbsem_ml_baseline"
        )
        self.assertEqual(
            baseline_report["support_classification"],
            "bounded_release_foundation",
        )
        bootstrap_report = next(
            row for row in report["tracks"] if row["track_id"] == "cbsem_bootstrap"
        )
        self.assertEqual(
            bootstrap_report["support_classification"],
            "implemented_candidate_unqualified",
        )
        preview_tracks = {
            row["track_id"]: row["ineligible_preview_versions"]
            for row in report["tracks"]
            if row["ineligible_preview_versions"]
        }
        self.assertEqual(
            preview_tracks,
            {
                "cbsem_bootstrap": ["cbsem_bootstrap_v1"],
                "cbsem_measurement_invariance": ["cbsem_invariance_v1"],
                "cbsem_multigroup": ["cbsem_multigroup_v1"],
            },
        )


if __name__ == "__main__":
    unittest.main()
