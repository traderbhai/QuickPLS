from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

import v255_evidence_bundle_builder as builder  # noqa: E402
import v255_product_completion_audit as audit  # noqa: E402


Mutation = Callable[[dict[str, Any], int], None]


def write_bytes(path: Path, payload: bytes) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload)
    return hashlib.sha256(payload).hexdigest()


def write_json(path: Path, payload: dict[str, Any]) -> str:
    encoded = (json.dumps(payload, indent=2) + "\n").encode()
    return write_bytes(path, encoded)


def capture(kind: str, capture_index: int, family: str, artifact: dict[str, str]) -> dict[str, Any]:
    source = "table_headers" if capture_index == 0 else "navigation"
    labels = {
        "selected_result": f"{kind} result",
        "document_tab": "Results",
        "navigation": [f"{family} navigation"],
        "visible_headings": [f"{family} heading"],
        "visible_result_table_ids": [f"{kind}-table"],
        "table_titles": [f"{family} table"],
        "table_headers": [f"{family} header"],
        "table_rows": [f"{family} row"],
        "chart_titles": [f"{family} chart"],
    }
    observed = labels[source][0]
    return {
        "status": "verified_current_ui_capture",
        "declared_identity": {
            "type": "canonical_result_document_id",
            "value": f"result-{kind}-{capture_index}",
        },
        "identity_verification": {"passed": True},
        "archive": dict(artifact),
        "screenshot": dict(artifact),
        "source_receipt": {
            **artifact,
            "declared_identity_directly_bound": True,
            "identity_recovered_from_archive": False,
        },
        "observed_results_labels": labels,
        "covers": [family],
        "cover_assertions": [
            {
                "family": family,
                "source": source,
                "matcher": "exact",
                "value": observed,
                "observed_json_pointer": (
                    f"/evidence/{capture_index}/observed_results_labels/{source}/0"
                ),
                "observed_value": observed,
                "passed": True,
            }
        ],
    }


def frozen_fixture(root: Path, mutation: Mutation | None = None) -> tuple[dict[str, Any], dict[str, Any]]:
    artifact_member = "artifacts/shared.bin"
    artifact_sha = write_bytes(root / artifact_member, b"frozen artifact")
    artifact = {"member": artifact_member, "sha256": artifact_sha}
    methods: list[dict[str, Any]] = []
    artifacts: list[dict[str, Any]] = []
    for ordinal in range(18):
        kind = f"method_{ordinal:02d}"
        families = [f"family_{ordinal}_a"]
        captures = [capture(kind, 0, families[0], artifact)]
        if ordinal == 0:
            families.append("family_0_b")
            captures.append(capture(kind, 1, families[1], artifact))
        receipt = {
            "schema_version": 2,
            "suite_id": builder.FROZEN_SUITE,
            "target_release": builder.TARGET,
            "status": "verified_current_ui_capture",
            "method_kind": kind,
            "evidence": captures,
        }
        if mutation is not None:
            mutation(receipt, ordinal)
        receipt_member = f"receipts/{kind}.json"
        receipt_sha = write_json(root / receipt_member, receipt)
        artifacts.append(
            {
                "member": receipt_member,
                "sha256": receipt_sha,
                "status": receipt.get("status"),
                "method_kind": receipt.get("method_kind"),
            }
        )
        methods.append(
            {
                "kind": kind,
                "status": "pending",
                "representative_results": families,
                "evidence": [],
            }
        )
    template = {"schema_version": 4, "status": "pending_collection", "methods": methods}
    aggregate = {
        "schema_version": 1,
        "suite_id": builder.FROZEN_SUITE,
        "target_release": builder.TARGET,
        "status": "passed",
        "failures": [],
        "console_errors": [],
        "method_receipts": artifacts,
    }
    return template, aggregate


class V255FrozenMultiRouteContractTests(unittest.TestCase):
    def test_builder_derives_multiple_top_level_index_entries_from_18_outer_receipts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            staging = Path(temporary)
            template, aggregate = frozen_fixture(staging)
            proposal = builder.derive_frozen(template, staging, aggregate)

        self.assertEqual(4, proposal["schema_version"])
        self.assertEqual(18, len(proposal["methods"]))
        first = proposal["methods"][0]
        self.assertEqual("verified", first["status"])
        self.assertEqual(2, len(first["evidence"]))
        self.assertEqual(
            {"method_kind", "canonical_result_id", "covers", "cover_assertions", "archive", "screenshot", "receipt"},
            set(first["evidence"][0]),
        )
        self.assertEqual("/evidence/0", first["evidence"][0]["receipt"]["evidence_json_pointer"])
        self.assertEqual(
            "/evidence/1/declared_identity/value",
            first["evidence"][1]["receipt"]["canonical_result_id_json_pointer"],
        )

    def test_builder_rejects_missing_extra_duplicate_and_unobserved_families(self) -> None:
        def missing(receipt: dict[str, Any], ordinal: int) -> None:
            if ordinal == 0:
                receipt["evidence"].pop()

        def extra(receipt: dict[str, Any], ordinal: int) -> None:
            if ordinal == 0:
                extra_capture = copy.deepcopy(receipt["evidence"][1])
                extra_capture["covers"] = ["undeclared"]
                extra_capture["cover_assertions"][0]["family"] = "undeclared"
                receipt["evidence"].append(extra_capture)

        def duplicate(receipt: dict[str, Any], ordinal: int) -> None:
            if ordinal == 0:
                receipt["evidence"][1]["covers"] = ["family_0_a"]
                receipt["evidence"][1]["cover_assertions"][0]["family"] = "family_0_a"

        def tampered_observation(receipt: dict[str, Any], ordinal: int) -> None:
            if ordinal == 0:
                receipt["evidence"][0]["cover_assertions"][0]["observed_value"] = "tampered"

        for name, mutation in {
            "missing": missing,
            "extra": extra,
            "duplicate": duplicate,
            "tampered_observation": tampered_observation,
        }.items():
            with self.subTest(mutation=name), tempfile.TemporaryDirectory() as temporary:
                staging = Path(temporary)
                template, aggregate = frozen_fixture(staging, mutation)
                with self.assertRaises(ValueError):
                    builder.derive_frozen(template, staging, aggregate)

    def test_builder_rejects_empty_table_rows_bad_pointer_identity_and_hash(self) -> None:
        def empty_rows(receipt: dict[str, Any], ordinal: int) -> None:
            if ordinal == 0:
                receipt["evidence"][0]["observed_results_labels"]["table_rows"] = []

        def bad_pointer(receipt: dict[str, Any], ordinal: int) -> None:
            if ordinal == 0:
                receipt["evidence"][0]["cover_assertions"][0]["observed_json_pointer"] = (
                    "/evidence/1/observed_results_labels/navigation/0"
                )

        def bad_identity(receipt: dict[str, Any], ordinal: int) -> None:
            if ordinal == 0:
                receipt["evidence"][0]["declared_identity"]["value"] = ""

        for name, mutation in {
            "empty_rows": empty_rows,
            "bad_pointer": bad_pointer,
            "bad_identity": bad_identity,
        }.items():
            with self.subTest(mutation=name), tempfile.TemporaryDirectory() as temporary:
                staging = Path(temporary)
                template, aggregate = frozen_fixture(staging, mutation)
                with self.assertRaises(ValueError):
                    builder.derive_frozen(template, staging, aggregate)

        with tempfile.TemporaryDirectory() as temporary:
            staging = Path(temporary)
            template, aggregate = frozen_fixture(staging)
            aggregate["method_receipts"][0]["sha256"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                builder.derive_frozen(template, staging, aggregate)

    def test_audit_recomputes_absolute_pointer_and_table_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            template, aggregate = frozen_fixture(Path(temporary))
            del template, aggregate
            receipt_path = Path(temporary) / "receipts/method_00.json"
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        self.assertTrue(audit.exact_frozen_capture_coverage(receipt, receipt["evidence"][0], 0))
        tampered = copy.deepcopy(receipt)
        tampered["evidence"][0]["cover_assertions"][0]["passed"] = False
        self.assertFalse(audit.exact_frozen_capture_coverage(tampered, tampered["evidence"][0], 0))
        tampered = copy.deepcopy(receipt)
        tampered["evidence"][0]["observed_results_labels"]["table_headers"] = []
        self.assertFalse(audit.exact_frozen_capture_coverage(tampered, tampered["evidence"][0], 0))

    def test_node_consumer_source_requires_v4_multiroute_bindings(self) -> None:
        source = (VALIDATION / "v255_method_evidence_crawler.mjs").read_text(encoding="utf-8")
        for fragment in (
            "archiveIndex.schema_version === 4",
            'typeof evidence.receipt?.evidence_json_pointer === "string"',
            'evidence.receipt.method_kind_json_pointer === "/method_kind"',
            "canonicalPointerBelongsToCapture",
            "receiptPayload?.schema_version === 2",
            "coveredFamilyCounts.get(family) === 1",
            "assertion?.observed_json_pointer",
            "compact(assertion?.observed_value) === observed",
            "labels.table_headers.some",
            "labels.table_rows.some",
            "presentationEvidenceContract(matrix, assertions)",
            'owner?.kind === "pls_bootstrap"',
            'evidence?.evidence_type === "component"',
            "presentationContract.passed",
        ):
            self.assertIn(fragment, source)

    def test_exact_fit_presentation_lane_is_separate_and_exact(self) -> None:
        matrix = json.loads(
            (VALIDATION / "v255_method_evidence_matrix.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertTrue(audit.exact_presentation_evidence_declarations(matrix))
        cbsem = next(method for method in matrix["methods"] if method["kind"] == "cbsem")
        self.assertEqual(
            ["parameters", "model fit", "identification", "bootstrap inference"],
            cbsem["result_families"],
        )
        frozen = json.loads(
            (VALIDATION / "v255_frozen_result_archive_index.json").read_text(
                encoding="utf-8"
            )
        )
        frozen_cbsem = next(method for method in frozen["methods"] if method["kind"] == "cbsem")
        self.assertEqual(cbsem["result_families"], frozen_cbsem["representative_results"])


if __name__ == "__main__":
    unittest.main()
