from __future__ import annotations

import hashlib
import json
import re
import subprocess
import unittest
import zipfile
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
INVENTORY_PATH = ROOT / "validation" / "v255_reusable_archive_inventory.json"
MATRIX_PATH = ROOT / "validation" / "v255_method_evidence_matrix.json"
NAMED_MANIFEST_PATH = ROOT / "validation" / "v255_named_case_manifest.json"

POSTHOC_KIND = "pls_posthoc_technical_minimum_sample_size"
ALLOWED_SOURCE_KINDS = {"inventory", "posthoc_supplement", "named_supplement"}
ALLOWED_OBSERVATION_SOURCES = {
    "navigation",
    "visible_headings",
    "visible_result_table_ids",
    "table_titles",
    "table_headers",
    "table_rows",
    "chart_titles",
}
ALLOWED_MATCHERS = {"exact", "contains"}
SAFE_DOM_ID = re.compile(r"^[a-zA-Z0-9:._-]+$")

EXPECTED_NAMED_BINDINGS = {
    "specialized_result:parallel mediation": "pls_algorithm",
    "specialized_result:multiple-mediation bootstrap": "pls_bootstrap",
    "specialized_result:three-way moderation": "pls_bootstrap",
    "specialized_result:first-stage moderated mediation": "pls_bootstrap",
    "specialized_result:HOC bootstrap": "pls_bootstrap",
    "specialized_result:recursive SEM point": "cbsem",
    "specialized_result:recursive SEM case bootstrap": "cbsem",
}


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def repo_path(relative: str) -> Path:
    path = (ROOT / relative).resolve()
    path.relative_to(ROOT.resolve())
    return path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_tracks(relative: str) -> bool:
    completed = subprocess.run(
        ["git", "ls-files", "--error-unmatch", "--", relative],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    return completed.returncode == 0


def resolve_json_pointer(document: Any, pointer: str) -> Any:
    if pointer == "":
        return document
    if not pointer.startswith("/"):
        raise AssertionError(f"Invalid JSON pointer: {pointer}")
    current = document
    for raw_segment in pointer[1:].split("/"):
        segment = raw_segment.replace("~1", "/").replace("~0", "~")
        if isinstance(current, list):
            current = current[int(segment)]
        elif isinstance(current, dict) and segment in current:
            current = current[segment]
        else:
            raise AssertionError(f"JSON pointer {pointer!r} stops at missing segment {segment!r}")
    return current


def archive_result_ids(path: Path, identity_type: str) -> set[str]:
    with zipfile.ZipFile(path) as archive:
        project = json.loads(archive.read("project.json"))
    if identity_type == "canonical_result_document_id":
        return {
            str(row.get("document_id") or row.get("canonical_document", {}).get("document_id"))
            for row in project.get("canonical_result_documents", [])
        }
    if identity_type == "schema5_result_run_id":
        return {str(row.get("id")) for row in project.get("results", [])}
    raise AssertionError(f"Unsupported result identity type: {identity_type}")


class V255ReusableArchiveInventoryContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.inventory = read_json(INVENTORY_PATH)
        cls.matrix = read_json(MATRIX_PATH)
        cls.named_manifest = read_json(NAMED_MANIFEST_PATH)

    def test_catalogue_and_route_family_union_are_exact(self) -> None:
        inventory = self.inventory
        rows = inventory["public_methods"]
        matrix_rows = self.matrix["methods"]
        matrix_families = {row["kind"]: row["result_families"] for row in matrix_rows}
        public_kinds = [row["public_kind"] for row in rows]

        self.assertEqual(inventory["schema"], "quickpls.v255.reusable_archive_inventory.v2")
        self.assertEqual(
            inventory["inventory_status"],
            "source_ready_static_routes_pending_dynamic_supplements_and_current_candidate_capture",
        )
        self.assertEqual(inventory["catalogue_size"], 18)
        self.assertTrue(inventory["paths_are_repo_relative"])
        self.assertTrue(inventory["hashes_computed"])
        self.assertEqual(len(rows), 18)
        self.assertEqual(len(set(public_kinds)), 18)
        self.assertEqual(set(public_kinds), set(matrix_families))

        reusable = [
            row for row in rows
            if row["reuse_state"] == "reusable_verified_prior_release"
            and row["new_scientific_run_required"] is False
        ]
        new_run = [row for row in rows if row["new_scientific_run_required"] is True]
        self.assertEqual(len(reusable), 17)
        self.assertEqual([row["public_kind"] for row in new_run], [POSTHOC_KIND])

        route_ids: list[str] = []
        capture_ids: list[str] = []
        covered: dict[str, list[str]] = defaultdict(list)
        for route in inventory["coverage_routes"]:
            route_id = route["route_id"]
            route_ids.append(route_id)
            self.assertIn(route["public_kind"], matrix_families, route_id)
            self.assertIn(route["source"]["kind"], ALLOWED_SOURCE_KINDS, route_id)
            self.assertTrue(route["captures"], route_id)

            for capture in route["captures"]:
                capture_id = capture["capture_id"]
                capture_ids.append(capture_id)
                self.assertTrue(capture["covers"], capture_id)
                self.assertEqual(len(capture["covers"]), len(set(capture["covers"])), capture_id)
                self.assertRegex(capture["activate"]["result_tree_item_id"], SAFE_DOM_ID, capture_id)
                for optional_id in ("table_id", "chart_id"):
                    if optional_id in capture["activate"]:
                        self.assertRegex(capture["activate"][optional_id], SAFE_DOM_ID, capture_id)

                observations = capture["observations"]
                self.assertEqual(len(observations), len(capture["covers"]), capture_id)
                self.assertEqual(
                    Counter(observation["family"] for observation in observations),
                    Counter(capture["covers"]),
                    capture_id,
                )
                for observation in observations:
                    self.assertIn(observation["source"], ALLOWED_OBSERVATION_SOURCES, capture_id)
                    self.assertIn(observation["matcher"], ALLOWED_MATCHERS, capture_id)
                    self.assertTrue(observation["value"].strip(), capture_id)
                    if observation["source"].startswith("table_"):
                        self.assertIn("table_id", capture["activate"], capture_id)
                    if observation["source"] == "chart_titles":
                        self.assertTrue(
                            "table_id" in capture["activate"] or "chart_id" in capture["activate"],
                            capture_id,
                        )
                    covered[route["public_kind"]].append(observation["family"])

        self.assertEqual(len(route_ids), len(set(route_ids)))
        self.assertEqual(len(capture_ids), len(set(capture_ids)))
        for kind, expected_families in matrix_families.items():
            self.assertEqual(Counter(covered[kind]), Counter(expected_families), kind)

        cbsem_families = covered["cbsem"]
        self.assertEqual(
            Counter(cbsem_families),
            Counter(["parameters", "model fit", "identification", "bootstrap inference"]),
        )
        self.assertFalse(any("exact-fit" in family.lower() for family in cbsem_families))

    def test_static_sources_are_curated_tracked_hashed_and_receipt_bound(self) -> None:
        inventory = self.inventory
        rows = [
            row for row in inventory["public_methods"]
            if row["new_scientific_run_required"] is False
        ] + inventory["specialized_feature_archives"]

        for row in rows:
            label = row.get("feature", row["public_kind"])
            relative = row["archive_path"]
            self.assertTrue(
                relative.startswith("validation/fixtures/v255/archives/"),
                f"{label} is not a curated v255 fixture: {relative}",
            )
            archive = repo_path(relative)
            self.assertTrue(archive.is_file(), label)
            self.assertTrue(git_tracks(relative), f"{label} archive is not Git-tracked: {relative}")
            self.assertRegex(row["archive_sha256"], r"^[0-9a-f]{64}$", label)
            self.assertEqual(sha256(archive), row["archive_sha256"], label)
            self.assertEqual(archive.stat().st_size, row["archive_size_bytes"], label)
            identity = row["result_identity"]
            self.assertIn(identity["value"], archive_result_ids(archive, identity["type"]), label)

            receipt = row["prior_receipt"]
            self.assertTrue(git_tracks(receipt["path"]), f"{label} receipt is not Git-tracked")
            receipt_document = read_json(repo_path(receipt["path"]))
            observed = resolve_json_pointer(receipt_document, receipt["json_pointer"])
            self.assertNotIn("identity_recovered_from_archive_not_pointer", receipt, label)
            self.assertEqual(observed, identity["value"], label)

        sections = {
            "public_methods": ("public_kind", inventory["public_methods"]),
            "specialized_feature_archives": ("feature", inventory["specialized_feature_archives"]),
        }
        for route in inventory["coverage_routes"]:
            if route["source"]["kind"] != "inventory":
                continue
            section = route["source"]["inventory_section"]
            key_name, source_rows = sections[section]
            key_value = route["source"]["inventory_key"]
            matches = [row for row in source_rows if row[key_name] == key_value]
            self.assertEqual(len(matches), 1, route["route_id"])
            source_row = matches[0]
            self.assertEqual(source_row["public_kind"], route["public_kind"], route["route_id"])
            for field in (
                "archive_path",
                "archive_sha256",
                "result_identity",
                "scientific_identity",
                "prior_receipt",
            ):
                self.assertEqual(route[field], source_row[field], f"{route['route_id']} {field}")

    def test_dynamic_supplement_bindings_are_exact(self) -> None:
        routes = self.inventory["coverage_routes"]
        named_routes = [route for route in routes if route["source"]["kind"] == "named_supplement"]
        actual_bindings = {route["source"]["case_id"]: route["public_kind"] for route in named_routes}
        self.assertEqual(actual_bindings, EXPECTED_NAMED_BINDINGS)

        manifest_cases = {row["id"]: row for row in self.named_manifest["cases"]}
        for case_id, public_kind in EXPECTED_NAMED_BINDINGS.items():
            self.assertIn(case_id, manifest_cases)
            manifest_route = manifest_cases[case_id]["route"]
            self.assertEqual(manifest_route["archive_supplement_public_kind"], public_kind, case_id)

        dynamic_fields = {
            "archive_path",
            "archive_sha256",
            "result_identity",
            "scientific_identity",
            "prior_receipt",
        }
        for route in routes:
            source_kind = route["source"]["kind"]
            if source_kind not in {"named_supplement", "posthoc_supplement"}:
                continue
            self.assertTrue(dynamic_fields.isdisjoint(route), route["route_id"])

        posthoc_routes = [route for route in routes if route["source"]["kind"] == "posthoc_supplement"]
        self.assertEqual(len(posthoc_routes), 1)
        self.assertEqual(posthoc_routes[0]["public_kind"], POSTHOC_KIND)
        self.assertEqual(posthoc_routes[0]["source"], {"kind": "posthoc_supplement"})

    def test_high_risk_observations_match_current_render_producers(self) -> None:
        captures = {
            (route["route_id"], capture["capture_id"]): capture
            for route in self.inventory["coverage_routes"]
            for capture in route["captures"]
        }

        expected = {
            ("cbsem_recursive_point", "cbsem_parameters"): (
                "canonical:table:cbsem_general_sem_parameters",
                "cbsem_general_sem_parameters",
                {"parameters": ("table_titles", "exact", "CB-SEM parameter table")},
            ),
            ("cbsem_recursive_point", "cbsem_model_fit"): (
                "canonical:table:cbsem_general_sem_fit",
                "cbsem_general_sem_fit",
                {"model fit": ("table_titles", "exact", "CB-SEM model fit")},
            ),
            ("cbsem_recursive_point", "cbsem_identification"): (
                "canonical:table:cbsem_general_sem_identification",
                "cbsem_general_sem_identification",
                {"identification": ("table_titles", "exact", "CB-SEM identification evidence")},
            ),
            ("pls_bootstrap_higher_order", "pls_bootstrap_hoc_bootstrap"): (
                "canonical:table:general_sem_higher_order_bootstrap_receipt",
                "general_sem_higher_order_bootstrap_receipt",
                {"HOC bootstrap": ("table_titles", "exact", "Higher-order bootstrap receipt")},
            ),
            ("ipma_construct_indicator_map", "ipma_result_families"): (
                "ipma_constructs",
                "ipma_constructs",
                {
                    "construct importance-performance": ("navigation", "exact", "Construct importance and performance"),
                    "indicator performance": ("navigation", "exact", "Indicator performance"),
                    "map": ("chart_titles", "contains", "Importance-performance map for"),
                },
            ),
            ("pls_sample_size_power_simulation", "pls_power_curve_grid_intervals"): (
                "pls_power_by_sample_size",
                "pls_power_by_sample_size",
                {
                    "power curve": ("chart_titles", "exact", "Prospective PLS-SEM power by sample size"),
                    "sample-size grid": ("table_headers", "exact", "sample_size"),
                    "Wilson intervals": ("table_headers", "exact", "confidence_lower"),
                },
            ),
            ("pls_sample_size_power_simulation", "pls_power_simulation_receipt"): (
                "pls_power_run_provenance",
                "pls_power_run_provenance",
                {"simulation receipt": ("table_rows", "exact", "outcome_digest")},
            ),
            ("nca_ceiling_analysis", "nca_effects_lines_inference"): (
                "nca_ceiling_effects",
                "nca_ceiling_effects",
                {
                    "effect sizes": ("table_titles", "exact", "Ceiling effect sizes and permutation inference"),
                    "ceiling lines": ("chart_titles", "contains", "Necessary condition ceiling plot for"),
                    "permutation inference": ("table_headers", "exact", "Permutation p"),
                },
            ),
            ("nca_ceiling_analysis", "nca_bottleneck_table"): (
                "nca_bottlenecks",
                "nca_bottlenecks",
                {"bottleneck table": ("table_titles", "exact", "Observed-range bottlenecks")},
            ),
        }

        for key, (tree_id, table_id, observations) in expected.items():
            capture = captures[key]
            self.assertEqual(capture["activate"]["result_tree_item_id"], tree_id, key)
            self.assertEqual(capture["activate"]["table_id"], table_id, key)
            actual_observations = {
                row["family"]: (row["source"], row["matcher"], row["value"])
                for row in capture["observations"]
            }
            self.assertEqual(actual_observations, observations, key)

        producer_checks = {
            "crates/qpls-core/src/canonical_cbsem_general_sem_projection_v1.rs": [
                "CB-SEM parameter table",
                "CB-SEM model fit",
                "CB-SEM identification evidence",
            ],
            "src-tauri/src/recipe_v4_general_sem_canonical_result.rs": [
                "Higher-order bootstrap receipt",
            ],
            "src/native/nativeResults.ts": [
                "Construct importance and performance",
                "Indicator performance",
                "Importance-performance map",
                "Ceiling effect sizes and permutation inference",
                "Permutation p",
                "Observed-range bottlenecks",
            ],
            "src/native/NativeResultsSurface.tsx": [
                "Prospective PLS-SEM power by sample size",
                "Importance-performance map for",
                "Necessary condition ceiling plot for",
            ],
        }
        for relative, needles in producer_checks.items():
            source = repo_path(relative).read_text(encoding="utf-8")
            for needle in needles:
                self.assertIn(needle, source, f"{needle!r} is absent from {relative}")


if __name__ == "__main__":
    unittest.main()
