from __future__ import annotations

import base64
import binascii
import csv
import io
import json
import struct
import tempfile
import unittest
import zipfile
from pathlib import Path

if __package__:
    from .general_sem_rank0_export_semantic_readback import (
        METADATA_ID,
        semantic_readback,
    )
else:
    from general_sem_rank0_export_semantic_readback import (
        METADATA_ID,
        semantic_readback,
    )


RESULTS_ROOT = Path(__file__).resolve().parent / "results"


def _document():
    capability = {
        "registry_schema_version": 2,
        "capability_id": "smartpls.moderation",
        "cell_id": "qpls3.pls.general_sem_multiple_two_way_moderation_point",
        "capability_version": "general_sem_pls_multiple_two_way_moderation_point_v1",
    }
    provenance = {
        "run_id": "run-1",
        "project_id": "project-1",
        "model_id": "model-1",
        "model_digest": "a" * 64,
        "dataset_id": "dataset-1",
        "dataset_fingerprint": "b" * 64,
        "recipe_id": "recipe-1",
        "recipe_digest": "c" * 64,
        "capability_cell": capability,
        "method_version": "qpls.general-sem-pls.multiple-two-way.point.v1",
        "engine_version": "test",
        "seed": 7,
        "workers": 1,
        "started_at": "2026-08-19T00:00:00Z",
        "completed_at": "2026-08-19T00:00:01Z",
    }
    table = {
        "id": "effects",
        "title": "Effects",
        "columns": [{"id": "estimate", "label": "Estimate", "data_type": "number", "description": "Estimate."}],
        "rows": [{"id": "gamma", "cells": [{"kind": "number", "value": 0.25}]}],
        "footnote_ids": [],
        "capability_cells": [capability],
    }
    chart = {
        "id": "plot",
        "title": "Plot",
        "description": "Canonical plot.",
        "kind": "line",
        "series": [{"id": "estimate", "label": "Estimate", "points": [{"x": 1, "y": 0.25}]}],
        "source_table_id": "effects",
        "display": {},
    }
    return {
        "schema_version": 2,
        "document_id": "document-1",
        "title": "Result",
        "provenance": provenance,
        "capability_cells": [capability],
        "tables": [table],
        "charts": [chart],
    }


def _envelope(document, format_id):
    table_ids = [] if format_id in {"svg", "png"} else ["effects"]
    chart_ids = ["plot"] if format_id in {"html", "pdf", "svg", "png"} else []
    return {
        "schema_version": 2,
        "format": "quickpls.canonical-result-cross-format-export",
        "source": {
            "document_schema_version": 2,
            "document_id": document["document_id"],
            "semantic_projection_sha256": "d" * 64,
        },
        "title": document["title"],
        "provenance": document["provenance"],
        "capability_cells": document["capability_cells"],
        "selection": {"table_ids": table_ids, "chart_ids": chart_ids},
        "sections": [],
        "tables": document["tables"] if table_ids else [],
        "charts": document["charts"] if chart_ids else [],
        "notices": [],
        "exclusions": [],
        "footnotes": [],
        "presentation": {"precision": 4, "missing_value_label": "—"},
        "semantic_sha256": "e" * 64,
    }


def _encoded(envelope):
    return base64.b64encode(json.dumps(envelope, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).decode()


def _xml_sheet(rows):
    xml_rows = []
    for row_index, row in enumerate(rows, 1):
        cells = []
        for column_index, value in enumerate(row):
            column = chr(ord("A") + column_index)
            escaped = str(value).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
            cells.append(f'<c r="{column}{row_index}" t="inlineStr"><is><t>{escaped}</t></is></c>')
        xml_rows.append(f'<row r="{row_index}">{"".join(cells)}</row>')
    return f'<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{"".join(xml_rows)}</sheetData></worksheet>'


def _chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", binascii.crc32(kind + data) & 0xFFFFFFFF)


class GeneralSemRank0ExportSemanticReadbackTests(unittest.TestCase):
    def test_independently_reads_all_six_final_file_types(self) -> None:
        document = _document()
        with tempfile.TemporaryDirectory(dir=RESULTS_ROOT) as directory_value:
            directory = Path(directory_value)
            for format_id in ("csv", "xlsx", "html", "pdf", "svg", "png"):
                envelope = _envelope(document, format_id)
                encoded = _encoded(envelope)
                path = directory / f"result.{format_id}"
                if format_id == "csv":
                    stream = io.StringIO(newline="")
                    writer = csv.writer(stream, lineterminator="\r\n")
                    writer.writerows([
                        ["quickpls.canonical-result-cross-format-export", "2"],
                        [METADATA_ID, encoded],
                        [],
                        ["table_id", "effects"],
                        ["table_title", "Effects"],
                        ["column_ids", "row_id", "estimate"],
                        ["column_labels", "Row ID", "Estimate"],
                        ["column_types", "text", "number"],
                        ["row", "gamma", "0.25"],
                    ])
                    path.write_text(stream.getvalue(), encoding="utf-8", newline="")
                elif format_id == "xlsx":
                    manifest = [["quickpls_export_manifest_v2"], ["Status", "validated"], ["Warning", ""], [], ["Field ID", "Value"], [f"{METADATA_ID}.000000", encoded]]
                    table = [["effects"], ["Status", "validated"], ["Warning", ""], [], ["Row ID", "Estimate [estimate]"], ["gamma", "0.25"]]
                    with zipfile.ZipFile(path, "w") as archive:
                        archive.writestr("xl/worksheets/sheet1.xml", _xml_sheet(manifest))
                        archive.writestr("xl/worksheets/sheet2.xml", _xml_sheet(table))
                elif format_id == "html":
                    path.write_text(f'<!doctype html><script id="{METADATA_ID}">{encoded}</script><table data-canonical-table-id="effects"></table><figure data-canonical-chart-id="plot"></figure>', encoding="utf-8")
                elif format_id == "pdf":
                    path.write_bytes(f"%PDF-1.4\n%QPLS-V2:{encoded}\neffects plot\n%%EOF\n".encode("latin-1"))
                elif format_id == "svg":
                    path.write_text(f'<svg><metadata id="{METADATA_ID}">{encoded}</metadata><g data-canonical-chart-id="plot"></g></svg>', encoding="utf-8")
                else:
                    text = [
                        ("quickpls.semantic.v2", encoded),
                        ("Document ID", document["document_id"]),
                        ("Chart ID", "plot"),
                        ("Run ID", document["provenance"]["run_id"]),
                        ("Method version", document["provenance"]["method_version"]),
                        ("Dataset fingerprint", document["provenance"]["dataset_fingerprint"]),
                    ]
                    path.write_bytes(b"\x89PNG\r\n\x1a\n" + b"".join(_chunk(b"tEXt", key.encode("latin-1") + b"\0" + value.encode("latin-1")) for key, value in text) + _chunk(b"IEND", b""))
                with self.subTest(format=format_id):
                    result = semantic_readback(path, format_id, document)
                    self.assertTrue(result["passed"])
                    self.assertEqual(result["document_id"], "document-1")

    def test_tampered_visible_csv_value_fails(self) -> None:
        document = _document()
        envelope = _envelope(document, "csv")
        with tempfile.TemporaryDirectory(dir=RESULTS_ROOT) as directory_value:
            path = Path(directory_value) / "result.csv"
            path.write_text(f"{METADATA_ID},{_encoded(envelope)}\r\ntable_id,effects\r\na\r\nb\r\nc\r\nrow,gamma,0.26\r\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "rendered surface"):
                semantic_readback(path, "csv", document)


if __name__ == "__main__":
    unittest.main()
