#!/usr/bin/env python3
"""Independent semantic readback of final native six-format export bytes."""

from __future__ import annotations

import base64
import binascii
import csv
import hashlib
import io
import json
import re
import struct
import zipfile
from pathlib import Path
from typing import Any, Mapping
from xml.etree import ElementTree


METADATA_ID = "quickpls-canonical-semantic-export-v2"
EXPORT_FORMAT = "quickpls.canonical-result-cross-format-export"
SHA256 = re.compile(r"^[0-9a-f]{64}$")
DERIVED_CHARTS = {
    "quickpls_export_specific_indirect_effect_estimates_v2": (
        "general_sem_specific_indirect_effects",
        "Specific indirect effect estimates",
    ),
    "quickpls_export_aggregate_effect_estimates_v2": (
        "general_sem_aggregate_effects",
        "Aggregate effect estimates",
    ),
}


class SemanticReadbackError(ValueError):
    pass


def _stable_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def _digest(value: Any) -> str:
    return hashlib.sha256(_stable_bytes(value)).hexdigest()


def _decode_envelope(encoded: str) -> Mapping[str, Any]:
    try:
        compact = "".join(encoded.split())
        raw = base64.b64decode(compact, validate=True)
        if base64.b64encode(raw).decode("ascii") != compact:
            raise SemanticReadbackError("semantic payload base64 is not canonical")
        value = json.loads(raw)
    except (ValueError, UnicodeError, json.JSONDecodeError, binascii.Error) as error:
        raise SemanticReadbackError(f"semantic payload cannot be decoded: {error}") from error
    if not isinstance(value, Mapping):
        raise SemanticReadbackError("semantic payload is not one JSON object")
    return value


def _xlsx_rows(path: Path) -> dict[str, list[list[str]]]:
    with zipfile.ZipFile(path, "r") as archive:
        if archive.testzip() is not None:
            raise SemanticReadbackError("XLSX contains a corrupt ZIP member")
        names = set(archive.namelist())
        shared: list[str] = []
        if "xl/sharedStrings.xml" in names:
            root = ElementTree.fromstring(archive.read("xl/sharedStrings.xml"))
            for item in root.iter():
                if item.tag.rsplit("}", 1)[-1] == "si":
                    shared.append("".join(
                        child.text or ""
                        for child in item.iter()
                        if child.tag.rsplit("}", 1)[-1] == "t"
                    ))
        result: dict[str, list[list[str]]] = {}
        for name in sorted(entry for entry in names if re.fullmatch(r"xl/worksheets/sheet[0-9]+\.xml", entry)):
            root = ElementTree.fromstring(archive.read(name))
            rows: list[list[str]] = []
            for row in (node for node in root.iter() if node.tag.rsplit("}", 1)[-1] == "row"):
                values: dict[int, str] = {}
                for cell in (node for node in row if node.tag.rsplit("}", 1)[-1] == "c"):
                    reference = cell.attrib.get("r", "A1")
                    letters = re.match(r"[A-Z]+", reference)
                    if not letters:
                        raise SemanticReadbackError("XLSX cell reference is invalid")
                    column = 0
                    for character in letters.group(0):
                        column = column * 26 + ord(character) - 64
                    cell_type = cell.attrib.get("t")
                    value_node = next((node for node in cell if node.tag.rsplit("}", 1)[-1] == "v"), None)
                    if cell_type == "inlineStr":
                        value = "".join(
                            node.text or ""
                            for node in cell.iter()
                            if node.tag.rsplit("}", 1)[-1] == "t"
                        )
                    else:
                        raw = value_node.text if value_node is not None and value_node.text is not None else ""
                        if cell_type == "s":
                            try:
                                value = shared[int(raw)]
                            except (ValueError, IndexError) as error:
                                raise SemanticReadbackError("XLSX shared-string index is invalid") from error
                        else:
                            value = raw
                    values[column - 1] = value
                width = max(values, default=-1) + 1
                rows.append([values.get(index, "") for index in range(width)])
            title = rows[0][0] if rows and rows[0] else name
            if title in result:
                raise SemanticReadbackError(f"XLSX duplicates worksheet identity {title}")
            result[title] = rows
        return result


def _extract(path: Path, format_id: str) -> tuple[Mapping[str, Any], Any]:
    raw = path.read_bytes()
    if format_id == "csv":
        rows = list(csv.reader(io.StringIO(raw.decode("utf-8"))))
        matches = [row[1] for row in rows if len(row) == 2 and row[0] == METADATA_ID]
        if len(matches) != 1:
            raise SemanticReadbackError("CSV semantic manifest is not unique")
        return _decode_envelope(matches[0]), rows
    if format_id in {"html", "svg"}:
        text = raw.decode("utf-8")
        pattern = rf'<(?:script|metadata)[^>]*id=["\']{re.escape(METADATA_ID)}["\'][^>]*>([^<]+)</(?:script|metadata)>'
        matches = re.findall(pattern, text)
        if len(matches) != 1:
            raise SemanticReadbackError(f"{format_id.upper()} semantic manifest is not unique")
        return _decode_envelope(matches[0]), text
    if format_id == "pdf":
        if not raw.startswith(b"%PDF-") or not raw.endswith(b"%%EOF\n"):
            raise SemanticReadbackError("PDF byte signature is invalid")
        text = raw.decode("latin-1")
        matches = re.findall(r"^%QPLS-V2:(.+)$", text, flags=re.MULTILINE)
        if not matches:
            raise SemanticReadbackError("PDF semantic manifest is missing")
        return _decode_envelope("".join(matches)), text
    if format_id == "png":
        if not raw.startswith(b"\x89PNG\r\n\x1a\n"):
            raise SemanticReadbackError("PNG byte signature is invalid")
        offset = 8
        text_chunks: dict[str, str] = {}
        saw_iend = False
        while offset + 12 <= len(raw):
            length = struct.unpack(">I", raw[offset : offset + 4])[0]
            kind = raw[offset + 4 : offset + 8]
            end = offset + 12 + length
            if end > len(raw):
                raise SemanticReadbackError("PNG chunk leaves file bounds")
            data = raw[offset + 8 : offset + 8 + length]
            recorded_crc = struct.unpack(">I", raw[offset + 8 + length : end])[0]
            if binascii.crc32(kind + data) & 0xFFFFFFFF != recorded_crc:
                raise SemanticReadbackError("PNG chunk CRC is invalid")
            if kind == b"tEXt":
                key, separator, value = data.partition(b"\0")
                if not separator:
                    raise SemanticReadbackError("PNG tEXt chunk is invalid")
                decoded_key = key.decode("latin-1")
                if decoded_key in text_chunks:
                    raise SemanticReadbackError(f"PNG duplicates tEXt key {decoded_key}")
                text_chunks[decoded_key] = value.decode("latin-1")
            if kind == b"IEND":
                saw_iend = end == len(raw)
                break
            offset = end
        if not saw_iend or "quickpls.semantic.v2" not in text_chunks:
            raise SemanticReadbackError("PNG terminal or semantic chunk is missing")
        return _decode_envelope(text_chunks["quickpls.semantic.v2"]), text_chunks
    if format_id == "xlsx":
        if not zipfile.is_zipfile(path):
            raise SemanticReadbackError("XLSX is not a ZIP workbook")
        sheets = _xlsx_rows(path)
        manifest = sheets.get("quickpls_export_manifest_v2")
        if manifest is None:
            raise SemanticReadbackError("XLSX manifest worksheet is missing")
        chunks: dict[int, str] = {}
        for row in manifest:
            if len(row) >= 2 and row[0].startswith(f"{METADATA_ID}."):
                suffix = row[0].removeprefix(f"{METADATA_ID}.")
                if not re.fullmatch(r"[0-9]{6}", suffix):
                    raise SemanticReadbackError("XLSX semantic chunk identity is invalid")
                chunks[int(suffix)] = row[1]
        if not chunks or sorted(chunks) != list(range(len(chunks))):
            raise SemanticReadbackError("XLSX semantic chunks are incomplete")
        return _decode_envelope("".join(chunks[index] for index in range(len(chunks)))), sheets
    raise SemanticReadbackError(f"unsupported export format {format_id}")


def _semantic_cell(cell: Mapping[str, Any]) -> str:
    kind = cell.get("kind")
    if kind == "number":
        value = cell.get("value")
        if not isinstance(value, (int, float)) or isinstance(value, bool):
            raise SemanticReadbackError("canonical numeric cell is invalid")
        return "0" if value == 0 else str(value).lower().replace("e-0", "e-").replace("e+0", "e+")
    if kind == "boolean":
        return "true" if cell.get("value") is True else "false"
    if kind == "text":
        return str(cell.get("value"))
    if kind == "missing":
        return f"missing:{cell.get('reason')}"
    raise SemanticReadbackError("canonical cell kind is invalid")


def _expected_derived_chart(chart_id: str, document: Mapping[str, Any]) -> Mapping[str, Any]:
    table_id, title = DERIVED_CHARTS[chart_id]
    table = next((row for row in document.get("tables", []) if isinstance(row, Mapping) and row.get("id") == table_id), None)
    if not isinstance(table, Mapping):
        raise SemanticReadbackError(f"derived chart source table {table_id} is missing")
    columns = table.get("columns")
    if not isinstance(columns, list):
        raise SemanticReadbackError("derived chart source columns are missing")
    effect_index = next((index for index, row in enumerate(columns) if row.get("id") == "effect_id" and row.get("data_type") == "text"), -1)
    estimate_index = next((index for index, row in enumerate(columns) if row.get("id") == "estimate" and row.get("data_type") == "number"), -1)
    points = []
    for index, row in enumerate(table.get("rows", [])):
        cells = row.get("cells", [])
        effect, estimate = cells[effect_index], cells[estimate_index]
        points.append({"x": index + 1, "y": estimate["value"], "label": effect["value"]})
    return {
        "id": chart_id,
        "title": title,
        "description": f"Export-only visual derived exactly from canonical table {table_id}. Effect index follows canonical row order; every point retains its effect_id and estimate without creating a new estimand.",
        "kind": "bar",
        "series": [{"id": "estimate", "label": "Estimate", "points": points}],
        "source_table_id": table_id,
        "display": {"show_legend": True, "show_values": True, "x_axis_label": "Effect index", "y_axis_label": "Estimate"},
    }


def semantic_readback(path: Path, format_id: str, document: Mapping[str, Any]) -> dict[str, Any]:
    envelope, surface = _extract(path, format_id)
    if envelope.get("schema_version") != 2 or envelope.get("format") != EXPORT_FORMAT:
        raise SemanticReadbackError("semantic envelope identity is invalid")
    semantic_sha = envelope.get("semantic_sha256")
    source = envelope.get("source")
    provenance = envelope.get("provenance")
    selection = envelope.get("selection")
    tables = envelope.get("tables")
    charts = envelope.get("charts")
    if (
        not isinstance(semantic_sha, str)
        or not SHA256.fullmatch(semantic_sha)
        or not isinstance(source, Mapping)
        or source.get("document_schema_version") != document.get("schema_version")
        or source.get("document_id") != document.get("document_id")
        or not isinstance(source.get("semantic_projection_sha256"), str)
        or not SHA256.fullmatch(source["semantic_projection_sha256"])
        or provenance != document.get("provenance")
        or not isinstance(selection, Mapping)
        or not isinstance(tables, list)
        or not isinstance(charts, list)
    ):
        raise SemanticReadbackError("semantic envelope does not bind canonical identity/provenance")
    table_ids = selection.get("table_ids")
    chart_ids = selection.get("chart_ids")
    if not isinstance(table_ids, list) or not isinstance(chart_ids, list):
        raise SemanticReadbackError("semantic selection is invalid")
    expected_tables = [row for row in document.get("tables", []) if row.get("id") in set(table_ids)]
    if [row.get("id") for row in expected_tables] != table_ids or tables != expected_tables:
        raise SemanticReadbackError("export tables/values differ from archived canonical tables")
    persisted = {row.get("id"): row for row in document.get("charts", []) if isinstance(row, Mapping)}
    expected_charts = []
    for chart_id in chart_ids:
        if chart_id in persisted:
            expected_charts.append(persisted[chart_id])
        elif chart_id in DERIVED_CHARTS:
            expected_charts.append(_expected_derived_chart(chart_id, document))
        else:
            raise SemanticReadbackError(f"export chart {chart_id} has no canonical authority")
    if charts != expected_charts:
        raise SemanticReadbackError("export charts/values differ from canonical or exact derived authority")
    if envelope.get("capability_cells") != document.get("capability_cells"):
        raise SemanticReadbackError("export capability inventory differs from archive")

    rendered_match = True
    if format_id == "csv":
        for table in tables:
            marker = ["table_id", table["id"]]
            if marker not in surface:
                rendered_match = False
                break
            start = surface.index(marker)
            expected_rows = [["row", row["id"], *[_semantic_cell(cell) for cell in row["cells"]]] for row in table["rows"]]
            if surface[start + 5 : start + 5 + len(expected_rows)] != expected_rows:
                rendered_match = False
                break
    elif format_id == "xlsx":
        for table in tables:
            sheet = surface.get(table["id"])
            expected_rows = [[row["id"], *[_semantic_cell(cell) for cell in row["cells"]]] for row in table["rows"]]
            if not isinstance(sheet, list) or sheet[5 : 5 + len(expected_rows)] != expected_rows:
                rendered_match = False
                break
    elif format_id in {"html", "svg"}:
        rendered_match = all(f'data-canonical-table-id="{value}"' in surface for value in table_ids) and all(
            f'data-canonical-chart-id="{value}"' in surface for value in chart_ids
        )
    elif format_id == "pdf":
        rendered_match = all(value in surface for value in [*table_ids, *chart_ids])
    elif format_id == "png":
        rendered_match = (
            surface.get("Document ID") == document.get("document_id")
            and surface.get("Run ID") == provenance.get("run_id")
            and surface.get("Method version") == provenance.get("method_version")
            and surface.get("Dataset fingerprint") == provenance.get("dataset_fingerprint")
            and surface.get("Chart ID") == (chart_ids[0] if len(chart_ids) == 1 else None)
        )
    if not rendered_match:
        raise SemanticReadbackError(f"{format_id.upper()} rendered surface differs from semantic envelope")
    value_projection = {"tables": tables, "charts": charts}
    return {
        "schema_version": 1,
        "evidence_kind": "general_sem_rank0_export_semantic_readback",
        "format": format_id,
        "document_id": source["document_id"],
        "run_id": provenance["run_id"],
        "method_version": provenance["method_version"],
        "dataset_fingerprint": provenance["dataset_fingerprint"],
        "semantic_sha256": semantic_sha,
        "table_ids": table_ids,
        "chart_ids": chart_ids,
        "canonical_values_sha256": _digest(value_projection),
        "rendered_surface_match": True,
        "canonical_match": True,
        "passed": True,
    }

