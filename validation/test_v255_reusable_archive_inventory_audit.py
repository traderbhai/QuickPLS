from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

import v255_product_completion_audit as audit  # noqa: E402


POSTHOC = "pls_posthoc_technical_minimum_sample_size"


def write_bytes(root: Path, relative: str, payload: bytes) -> tuple[str, int]:
    target = root / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(payload)
    return hashlib.sha256(payload).hexdigest(), len(payload)


def write_json(root: Path, relative: str, payload: object) -> None:
    target = root / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(payload) + "\n", encoding="utf-8")


def source_fixture(root: Path) -> tuple[dict[str, object], dict[str, object], dict[str, object]]:
    ordinary_kinds = [f"method_{index:02d}" for index in range(17)]
    kinds = [*ordinary_kinds, POSTHOC]
    named_specs = [
        (f"named_case_{index}", ordinary_kinds[index]) for index in range(7)
    ]
    named_by_kind = {kind: case_id for case_id, kind in named_specs}
    matrix_methods: list[dict[str, object]] = []
    public_methods: list[dict[str, object]] = []
    coverage_routes: list[dict[str, object]] = []

    for index, kind in enumerate(kinds):
        family = f"family_{index:02d}"
        matrix_methods.append({"kind": kind, "result_families": [family]})
        capture = {
            "capture_id": f"capture_{index:02d}",
            "covers": [family],
            "activate": {
                "result_tree_item_id": f"table_{index:02d}",
                "table_id": f"table_{index:02d}",
            },
            "observations": [
                {
                    "family": family,
                    "source": "table_titles",
                    "matcher": "exact",
                    "value": f"Table {index:02d}",
                }
            ],
        }
        base_route = {
            "route_id": f"route_{index:02d}",
            "public_kind": kind,
            "captures": [capture],
        }
        if kind == POSTHOC:
            public_methods.append(
                {
                    "public_kind": POSTHOC,
                    "reuse_state": "no_packaged_completed_result_found",
                    "archive_path": None,
                    "result_identity": None,
                    "scientific_identity": {
                        "method_version": (
                            "pls_posthoc_technical_minimum_sample_size_v2"
                        ),
                        "qualification_path": (
                            "validation/qualification_v2/"
                            "pls_posthoc_technical_minimum_sample_size_v2."
                            "qualification.json"
                        ),
                    },
                    "prior_receipt": None,
                    "prior_screenshots": [],
                    "source_release": None,
                    "prior_verification_status": (
                        "scientific_qualification_only_no_packaged_result"
                    ),
                    "current_ui_capture_required": True,
                    "new_scientific_run_required": True,
                }
            )
            coverage_routes.append(
                {
                    **base_route,
                    "source": {"kind": "posthoc_supplement"},
                }
            )
            continue

        archive_path = f"fixtures/{kind}.qpls"
        receipt_path = f"receipts/{kind}.json"
        identity = {
            "type": "schema5_result_run_id",
            "value": f"result_{index:02d}",
        }
        scientific_identity = {"method_version": f"method_v{index:02d}"}
        prior_receipt = {
            "path": receipt_path,
            "json_pointer": "/result_id",
            "verification_status": "passed",
        }
        archive_hash, archive_size = write_bytes(
            root, archive_path, f"archive-{index:02d}".encode()
        )
        write_json(root, receipt_path, {"result_id": identity["value"]})
        public_methods.append(
            {
                "public_kind": kind,
                "archive_path": archive_path,
                "archive_sha256": archive_hash,
                "archive_size_bytes": archive_size,
                "result_identity": identity,
                "scientific_identity": scientific_identity,
                "prior_receipt": prior_receipt,
            }
        )
        if kind in named_by_kind:
            coverage_routes.append(
                {
                    **base_route,
                    "source": {
                        "kind": "named_supplement",
                        "case_id": named_by_kind[kind],
                    },
                }
            )
            continue
        coverage_routes.append(
            {
                **base_route,
                "source": {
                    "kind": "inventory",
                    "inventory_section": "public_methods",
                    "inventory_key": kind,
                },
                "archive_path": archive_path,
                "archive_sha256": archive_hash,
                "result_identity": identity,
                "scientific_identity": scientific_identity,
                "prior_receipt": prior_receipt,
            }
        )

    matrix = {"methods": matrix_methods}
    inventory = {
        "schema": audit.REUSABLE_ARCHIVE_INVENTORY_SCHEMA,
        "public_methods": public_methods,
        "specialized_feature_archives": [],
        "coverage_routes": coverage_routes,
    }
    manifest = {
        "cases": [
            {
                "id": case_id,
                "route": {"archive_supplement_public_kind": kind},
            }
            for case_id, kind in named_specs
        ]
    }
    return matrix, inventory, manifest


def evaluate(
    root: Path,
    matrix: dict[str, object],
    inventory: dict[str, object],
    manifest: dict[str, object],
    *,
    tracked: bool = True,
) -> dict[str, bool]:
    with (
        patch.object(audit, "ROOT", root),
        patch.object(
            audit,
            "git_tracked_repo_source_files",
            return_value=tracked,
        ),
    ):
        return audit.reusable_archive_inventory_v2_source_checks(
            matrix, inventory, manifest
        )


class V255ReusableArchiveInventoryAuditTests(unittest.TestCase):
    def test_valid_inventory_passes_every_source_stage_check(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            checks = evaluate(root, matrix, inventory, manifest)
        self.assertTrue(checks)
        self.assertTrue(all(checks.values()), checks)

    def test_schema_family_and_identifier_mutations_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            inventory["schema"] = "quickpls.v255.reusable_archive_inventory.v1"
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks["reusable_archive_inventory_uses_exact_v2_schema"]
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            capture = inventory["coverage_routes"][0]["captures"][0]
            capture["covers"].append("undeclared_family")
            capture["observations"].append(
                {
                    "family": "undeclared_family",
                    "source": "navigation",
                    "matcher": "exact",
                    "value": "Undeclared family",
                }
            )
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_has_exact_18_method_family_union"
                ]
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            routes = inventory["coverage_routes"]
            routes[1]["route_id"] = routes[0]["route_id"]
            routes[1]["captures"][0]["capture_id"] = routes[0]["captures"][0][
                "capture_id"
            ]
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_route_and_capture_ids_are_unique"
                ]
            )

    def test_source_activation_and_observation_schema_mutations_fail(self) -> None:
        mutations = (
            lambda route: route["source"].__setitem__("unexpected", True),
            lambda route: route["captures"][0]["activate"].__setitem__(
                "result_tree_item_id", "unsafe id"
            ),
            lambda route: route["captures"][0]["observations"][0].__setitem__(
                "unexpected", True
            ),
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                matrix, inventory, manifest = source_fixture(root)
                mutation(inventory["coverage_routes"][0])
                checks = evaluate(root, matrix, inventory, manifest)
                self.assertFalse(
                    checks[
                        "reusable_archive_inventory_route_capture_observation_schema_is_exact"
                    ]
                )

    def test_static_equality_hash_pointer_and_git_tracking_are_independent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            static_route = next(
                route
                for route in inventory["coverage_routes"]
                if route["source"]["kind"] == "inventory"
            )
            static_route["scientific_identity"] = {"method_version": "tampered"}
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_static_routes_equal_source_rows"
                ]
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            static_route = next(
                route
                for route in inventory["coverage_routes"]
                if route["source"]["kind"] == "inventory"
            )
            (root / static_route["archive_path"]).write_bytes(b"tampered")
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_static_archives_match_size_and_sha256"
                ]
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            static_route = next(
                route
                for route in inventory["coverage_routes"]
                if route["source"]["kind"] == "inventory"
            )
            write_json(
                root,
                static_route["prior_receipt"]["path"],
                {"result_id": "different"},
            )
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_prior_receipts_bind_declared_identity"
                ]
            )
            checks = evaluate(root, matrix, inventory, manifest, tracked=False)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_static_artifacts_are_git_tracked"
                ]
            )

    def test_unreferenced_static_row_is_still_hash_and_pointer_bound(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            named_kinds = {
                route["public_kind"]
                for route in inventory["coverage_routes"]
                if route["source"]["kind"] == "named_supplement"
            }
            unreferenced_row = next(
                row
                for row in inventory["public_methods"]
                if row["public_kind"] in named_kinds
            )
            (root / unreferenced_row["archive_path"]).write_bytes(b"tampered")
            write_json(
                root,
                unreferenced_row["prior_receipt"]["path"],
                {"result_id": "different"},
            )
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_static_archives_match_size_and_sha256"
                ]
            )
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_prior_receipts_bind_declared_identity"
                ]
            )

    def test_named_and_posthoc_dynamic_routes_must_be_exact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            named_route = next(
                route
                for route in inventory["coverage_routes"]
                if route["source"]["kind"] == "named_supplement"
            )
            named_route["source"]["case_id"] = "wrong_case"
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_has_exact_7_named_manifest_routes"
                ]
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            posthoc_route = next(
                route
                for route in inventory["coverage_routes"]
                if route["source"]["kind"] == "posthoc_supplement"
            )
            posthoc_route["source"]["unexpected"] = True
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks["reusable_archive_inventory_has_exact_one_posthoc_route"]
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            posthoc_row = next(
                row
                for row in inventory["public_methods"]
                if row["public_kind"] == POSTHOC
            )
            posthoc_row["archive_path"] = "fixtures/fabricated.qpls"
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_posthoc_public_row_is_exact_dynamic_exception"
                ]
            )

    def test_archive_recovery_flag_cannot_replace_direct_receipt_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            matrix, inventory, manifest = source_fixture(root)
            static_route = next(
                route
                for route in inventory["coverage_routes"]
                if route["source"]["kind"] == "inventory"
            )
            source_row = next(
                row
                for row in inventory["public_methods"]
                if row["public_kind"] == static_route["public_kind"]
            )
            recovered_receipt = {
                **static_route["prior_receipt"],
                "json_pointer": "/evidence",
                "identity_recovered_from_archive_not_pointer": True,
            }
            static_route["prior_receipt"] = copy.deepcopy(recovered_receipt)
            source_row["prior_receipt"] = copy.deepcopy(recovered_receipt)
            write_json(
                root,
                recovered_receipt["path"],
                {"evidence": {"passed": True}},
            )
            checks = evaluate(root, matrix, inventory, manifest)
            self.assertTrue(
                checks[
                    "reusable_archive_inventory_static_routes_equal_source_rows"
                ]
            )
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_route_capture_observation_schema_is_exact"
                ]
            )
            self.assertFalse(
                checks[
                    "reusable_archive_inventory_prior_receipts_bind_declared_identity"
                ]
            )


if __name__ == "__main__":
    unittest.main()
