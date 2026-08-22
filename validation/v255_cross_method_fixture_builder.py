#!/usr/bin/env python3
"""Materialize the small, deterministic QuickPLS 2.55 cross-method fixtures.

Only Python's standard library is used.  The four import files contain the
same three numeric variables and four cases.  Project fixtures are byte-for-
byte copies of committed archives, except for the future-schema fixture whose
manifest schema version is raised to 7 without changing compatible payloads.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import shutil
import struct
import zipfile
from pathlib import Path
from typing import Iterable


SUITE_ID = "quickpls_v255_cross_method_fixture_builder_v1"
ZIP_TIME = (1980, 1, 1, 0, 0, 0)


def fail(message: str) -> None:
    raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def under(root: Path, candidate: Path) -> bool:
    try:
        candidate.resolve().relative_to(root.resolve())
        return True
    except ValueError:
        return False


def zip_write(archive: zipfile.ZipFile, name: str, payload: bytes, compress: int) -> None:
    info = zipfile.ZipInfo(name, ZIP_TIME)
    info.create_system = 0
    info.external_attr = 0
    info.compress_type = compress
    archive.writestr(info, payload)


def read_rows(source: Path) -> tuple[list[str], list[list[float]]]:
    with source.open("r", encoding="utf-8", newline="") as stream:
        rows = list(csv.reader(stream))
    if rows != [
        ["x", "y", "z"],
        ["1", "2", "3"],
        ["2", "4", "6"],
        ["3", "6", "9"],
        ["4", "8", "12"],
    ]:
        fail("The committed numeric CSV no longer equals the frozen fixture table.")
    return rows[0], [[float(value) for value in row] for row in rows[1:]]


def xml_escape(value: str) -> str:
    return value.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def write_xlsx(path: Path, columns: list[str], rows: list[list[float]]) -> None:
    header = "".join(
        f'<c r="{chr(65 + index)}1" t="inlineStr"><is><t>{xml_escape(name)}</t></is></c>'
        for index, name in enumerate(columns)
    )
    body = []
    for row_index, row in enumerate(rows, 2):
        cells = "".join(
            f'<c r="{chr(65 + column_index)}{row_index}"><v>{value:g}</v></c>'
            for column_index, value in enumerate(row)
        )
        body.append(f'<row r="{row_index}">{cells}</row>')
    sheet = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
        f'<dimension ref="A1:C{len(rows) + 1}"/><sheetData><row r="1">{header}</row>'
        f'{"".join(body)}</sheetData></worksheet>'
    ).encode()
    members = {
        "[Content_Types].xml": b'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>''',
        "_rels/.rels": b'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>''',
        "xl/workbook.xml": b'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>''',
        "xl/_rels/workbook.xml.rels": b'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>''',
        "xl/worksheets/sheet1.xml": sheet,
    }
    with zipfile.ZipFile(path, "x") as archive:
        for name in sorted(members):
            zip_write(archive, name, members[name], zipfile.ZIP_DEFLATED)


def write_ods(path: Path, columns: list[str], rows: list[list[float]]) -> None:
    header = "".join(
        f'<table:table-cell office:value-type="string"><text:p>{xml_escape(name)}</text:p></table:table-cell>'
        for name in columns
    )
    data_rows = []
    for row in rows:
        cells = "".join(
            f'<table:table-cell office:value-type="float" office:value="{value:g}"><text:p>{value:g}</text:p></table:table-cell>'
            for value in row
        )
        data_rows.append(f"<table:table-row>{cells}</table:table-row>")
    content = (
        '<?xml version="1.0" encoding="UTF-8"?>'
        '<office:document-content '
        'xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" '
        'xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" '
        'xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.2">'
        '<office:body><office:spreadsheet><table:table table:name="Data">'
        f'<table:table-row>{header}</table:table-row>{"".join(data_rows)}'
        '</table:table></office:spreadsheet></office:body></office:document-content>'
    ).encode()
    manifest = b'''<?xml version="1.0" encoding="UTF-8"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2"><manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/></manifest:manifest>'''
    with zipfile.ZipFile(path, "x") as archive:
        zip_write(archive, "mimetype", b"application/vnd.oasis.opendocument.spreadsheet", zipfile.ZIP_STORED)
        zip_write(archive, "META-INF/manifest.xml", manifest, zipfile.ZIP_DEFLATED)
        zip_write(archive, "content.xml", content, zipfile.ZIP_DEFLATED)


def packed_format(kind: int = 5, width: int = 8, decimals: int = 2) -> int:
    return (kind << 16) | (width << 8) | decimals


def fixed(value: str, width: int) -> bytes:
    encoded = value.encode("ascii")
    if len(encoded) > width:
        fail(f"SPSS fixed field is too long: {value}")
    return encoded + b" " * (width - len(encoded))


def write_sav(path: Path, columns: list[str], rows: list[list[float]]) -> None:
    # Minimal, uncompressed, little-endian SPSS system file.  This is the same
    # record layout consumed by qpls-data's pinned ambers reader: one numeric
    # slot per variable, no labels/missing declarations, then raw IEEE doubles.
    payload = bytearray()
    payload += b"$FL2"
    payload += fixed("@(#) SPSS DATA FILE QuickPLS v255 fixture", 60)
    payload += struct.pack("<iiiii", 2, len(columns), 0, 0, len(rows))
    payload += struct.pack("<d", 100.0)
    payload += fixed("01 Jan 80", 9) + fixed("00:00:00", 8)
    payload += fixed("QuickPLS 2.55 deterministic import fixture", 64)
    payload += b"\0\0\0"
    for name in columns:
        payload += struct.pack("<iiiiii", 2, 0, 0, 0, packed_format(), packed_format())
        payload += fixed(name.upper(), 8)
    payload += struct.pack("<ii", 999, 0)
    for row in rows:
        for value in row:
            payload += struct.pack("<d", value)
    path.write_bytes(payload)


def copy_new(source: Path, target: Path) -> None:
    if target.exists():
        fail(f"Refusing to overwrite fixture: {target}")
    shutil.copyfile(source, target)


def write_future(source: Path, target: Path) -> None:
    with zipfile.ZipFile(source, "r") as current, zipfile.ZipFile(target, "x") as future:
        names = current.namelist()
        if names.count("manifest.json") != 1 or names.count("project.json") != 1:
            fail("The future-source archive lacks one exact manifest/project pair.")
        for name in names:
            payload = current.read(name)
            if name == "manifest.json":
                manifest = json.loads(payload)
                if manifest.get("schema_version") != 6:
                    fail("The future-source archive must be schema 6.")
                manifest["schema_version"] = 7
                payload = (json.dumps(manifest, indent=2, ensure_ascii=False) + "\n").encode()
            zip_write(future, name, payload, zipfile.ZIP_DEFLATED)


def file_row(path: Path, role: str, source: Path | None = None) -> dict[str, object]:
    return {
        "role": role,
        "path": str(path),
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
        "source_path": str(source) if source else None,
        "source_sha256": sha256(source) if source else None,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--csv-source", type=Path, required=True)
    parser.add_argument("--legacy-source", type=Path, required=True)
    parser.add_argument("--schema5-source", type=Path, required=True)
    parser.add_argument("--schema6-source", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()

    output = args.output_dir.resolve()
    report = args.report.resolve()
    if output.exists() or report.exists():
        fail("Output directory and report must both be new.")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.mkdir()
    sources = [args.csv_source, args.legacy_source, args.schema5_source, args.schema6_source]
    sources = [item.resolve(strict=True) for item in sources]
    if any(not item.is_file() or item.is_symlink() for item in sources):
        fail("Every fixture source must be a non-symlink regular file.")
    columns, rows = read_rows(sources[0])

    csv_target = output / "quickpls-v255-import.csv"
    xlsx_target = output / "quickpls-v255-import.xlsx"
    sav_target = output / "quickpls-v255-import.sav"
    ods_target = output / "quickpls-v255-import.ods"
    legacy_target = output / "quickpls-v255-legacy-v4.qpls"
    schema5_autosave = output / "quickpls-v255-autosave-source.qpls"
    schema5_close = output / "quickpls-v255-unsaved-close-source.qpls"
    schema6_export = output / "quickpls-v255-export-source.qpls"
    future_target = output / "quickpls-v255-future-v7.qpls"
    copy_new(sources[0], csv_target)
    write_xlsx(xlsx_target, columns, rows)
    write_sav(sav_target, columns, rows)
    write_ods(ods_target, columns, rows)
    copy_new(sources[1], legacy_target)
    copy_new(sources[2], schema5_autosave)
    copy_new(sources[2], schema5_close)
    copy_new(sources[3], schema6_export)
    write_future(sources[3], future_target)

    rows_out = [
        file_row(csv_target, "import_csv", sources[0]),
        file_row(xlsx_target, "import_xlsx"),
        file_row(sav_target, "import_spss_sav"),
        file_row(ods_target, "import_ods"),
        file_row(legacy_target, "legacy_schema4", sources[1]),
        file_row(schema5_autosave, "autosave_schema5", sources[2]),
        file_row(schema5_close, "unsaved_close_schema5", sources[2]),
        file_row(schema6_export, "export_schema6", sources[3]),
        file_row(future_target, "future_schema7", sources[3]),
    ]
    payload = {
        "schema_version": 1,
        "suite_id": SUITE_ID,
        "passed": True,
        "table": {"columns": columns, "rows": rows, "row_count": len(rows), "missing": 0},
        "files": rows_out,
    }
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
