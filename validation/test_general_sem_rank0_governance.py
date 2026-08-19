from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SIX_EXPORT_FORMATS = ["csv", "xlsx", "html", "pdf", "svg", "png"]

CELLS = {
    "mediation_point": {
        "capability_id": "smartpls.mediation",
        "cell_id": "qpls3.pls.mediation",
        "capability_version": "pls_mediation_v1",
        "analytical_method_version": "pls_mediation_v1",
        "method_manifest": "validation/methods/mediation_v1.manifest.json",
        "qualification_spec": "validation/qualification_v2/mediation_v1.qualification.json",
        "method_doc": "docs/methods/PLS_MEDIATION_V1.md",
    },
    "mediation_bootstrap": {
        "capability_id": "smartpls.mediation",
        "cell_id": "qpls3.pls.general_sem_multiple_mediation_bootstrap",
        "capability_version": "general_sem_pls_full_model_case_bootstrap_v1",
        "analytical_method_version": "general_sem_pls_full_model_case_bootstrap_v1",
        "method_manifest": (
            "validation/methods/"
            "general_sem_pls_multiple_mediation_bootstrap_v1.manifest.json"
        ),
        "qualification_spec": (
            "validation/qualification_v2/"
            "general_sem_pls_multiple_mediation_bootstrap_v1.qualification.json"
        ),
        "method_doc": (
            "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MEDIATION_BOOTSTRAP_V1.md"
        ),
    },
    "moderation_point": {
        "capability_id": "smartpls.moderation",
        "cell_id": "qpls3.pls.general_sem_multiple_two_way_moderation_point",
        "capability_version": "general_sem_pls_multiple_two_way_moderation_point_v1",
        "analytical_method_version": (
            "qpls.general-sem-pls.multiple-two-way.point.v1"
        ),
        "method_manifest": (
            "validation/methods/"
            "general_sem_pls_multiple_moderation_point_v1.manifest.json"
        ),
        "qualification_spec": (
            "validation/qualification_v2/"
            "general_sem_pls_multiple_moderation_point_v1.qualification.json"
        ),
        "method_doc": (
            "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_V1.md"
        ),
        "cell_manifest": (
            "validation/capabilities/"
            "general_sem_pls_multiple_moderation_point_v1.cell.manifest.json"
        ),
    },
    "moderation_bootstrap": {
        "capability_id": "smartpls.moderation",
        "cell_id": "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
        "capability_version": (
            "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1"
        ),
        "analytical_method_version": (
            "qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1"
        ),
        "method_manifest": (
            "validation/methods/"
            "general_sem_pls_multiple_moderation_bootstrap_v1.manifest.json"
        ),
        "qualification_spec": (
            "validation/qualification_v2/"
            "general_sem_pls_multiple_moderation_bootstrap_v1.qualification.json"
        ),
        "method_doc": (
            "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_V1.md"
        ),
        "cell_manifest": (
            "validation/capabilities/"
            "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1.cell.manifest.json"
        ),
    },
}


def _load(relative_path: str):
    return json.loads((ROOT / relative_path).read_text(encoding="utf-8"))


def _registry_cells(registry):
    return {
        cell["cell_id"]: (capability, cell)
        for capability in registry["capabilities"]
        for cell in capability["option_cells"]
        if cell["cell_id"] in {definition["cell_id"] for definition in CELLS.values()}
    }


class GeneralSemRank0GovernanceTests(unittest.TestCase):
    def test_unpromoted_cells_have_exact_identities_and_six_format_contracts(self) -> None:
        registry = _load("validation/capabilities/capability_registry_v2.json")
        registry_cells = _registry_cells(registry)
        self.assertEqual(set(registry_cells), {row["cell_id"] for row in CELLS.values()})

        for key, expected in CELLS.items():
            with self.subTest(cell=key):
                capability, registry_cell = registry_cells[expected["cell_id"]]
                self.assertEqual(capability["capability_id"], expected["capability_id"])
                self.assertEqual(capability["evidence_state"], "engine_only")
                self.assertEqual(capability["surface"], "labs")
                self.assertEqual(registry_cell["capability_version"], expected["capability_version"])
                self.assertEqual(registry_cell["coverage_state"], "partial")
                self.assertEqual(registry_cell["evidence_state"], "engine_only")
                self.assertEqual(registry_cell["surface"], "labs")

                manifest = _load(expected["method_manifest"])
                self.assertEqual(manifest["feature"]["id"], expected["cell_id"])
                self.assertEqual(
                    manifest["feature"]["method_version"],
                    expected["capability_version"],
                )
                self.assertEqual(manifest["qualification"]["declared_state"], "engine_only")
                self.assertEqual(
                    manifest["qualification"]["target_state"], "release_qualified"
                )
                self.assertEqual(
                    manifest["product_contract"]["export"]["formats"],
                    SIX_EXPORT_FORMATS,
                )

                spec = _load(expected["qualification_spec"])
                identity = spec["identity"]
                self.assertEqual(identity["capability_cell"]["capability_id"], expected["capability_id"])
                self.assertEqual(identity["capability_cell"]["cell_id"], expected["cell_id"])
                self.assertEqual(
                    identity["capability_cell"]["capability_version"],
                    expected["capability_version"],
                )
                self.assertEqual(identity["method_version"], expected["capability_version"])
                self.assertEqual(
                    identity["analytical_method_version"],
                    expected["analytical_method_version"],
                )
                self.assertEqual(spec["evidence_contract"]["receipts"], [])

                if expected.get("cell_manifest"):
                    cell_manifest = _load(expected["cell_manifest"])
                    self.assertEqual(cell_manifest["feature"]["id"], expected["cell_id"])
                    self.assertEqual(
                        cell_manifest["feature"]["analytical_method_version"],
                        expected["analytical_method_version"],
                    )
                    self.assertFalse(cell_manifest["qualification_ready"])
                    self.assertFalse(cell_manifest["promotion_allowed"])

                method_doc = (ROOT / expected["method_doc"]).read_text(encoding="utf-8").lower()
                for export_format in SIX_EXPORT_FORMATS:
                    self.assertIn(export_format, method_doc)

    def test_catalogue_package_and_complexity_pins_cover_the_exact_four_cells(self) -> None:
        expected_ids = {row["cell_id"] for row in CELLS.values()}
        catalogue = _load("validation/quickpls_3_competitor_catalogue.json")
        catalogue_rows = {
            row["id"]: row
            for row in catalogue["methods"]
            if row["id"] in {"smartpls.mediation", "smartpls.moderation"}
        }
        self.assertEqual(set(catalogue_rows), {"smartpls.mediation", "smartpls.moderation"})
        for row in catalogue_rows.values():
            self.assertEqual(row["status"], "engine-preview")
        self.assertTrue(
            expected_ids.issubset(
                {
                    cell_id
                    for row in catalogue_rows.values()
                    for cell_id in row["quickpls_capability_ids"]
                }
            )
        )

        packaged = _load(
            "validation/capabilities/general_sem_rank0_packaged_acceptance_v1.manifest.json"
        )
        packaged_references = {
            variant["capability_reference"]["cell_id"]: variant["capability_reference"]
            for variant in packaged["variants"]
        }
        self.assertEqual(set(packaged_references), expected_ids)
        for export_format in SIX_EXPORT_FORMATS:
            self.assertIn(
                f"export_{export_format}", packaged["common_required_check_ids"]
            )

        registry = _load("validation/capabilities/capability_registry_v2.json")
        registry_cells = _registry_cells(registry)
        complexity = _load(
            "validation/capabilities/complexity_performance_profiles_v2.manifest.json"
        )["capability_budget_resolution"]
        family_defaults = {
            row["official_family"]: row["budget_class_id"]
            for row in complexity["family_defaults"]
        }
        exact_overrides = {
            (
                row["reference"]["capability_id"],
                row["reference"]["cell_id"],
                row["reference"]["capability_version"],
            ): row["budget_class_id"]
            for row in complexity["exact_overrides"]
        }
        for expected in CELLS.values():
            capability, _ = registry_cells[expected["cell_id"]]
            identity = (
                expected["capability_id"],
                expected["cell_id"],
                expected["capability_version"],
            )
            resolved = exact_overrides.get(
                identity, family_defaults[capability["official_family"]]
            )
            self.assertEqual(resolved, "compute_intensive")


if __name__ == "__main__":
    unittest.main()
