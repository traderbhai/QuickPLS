"""Prepare the official SmartPLS Corporate Reputation data for parity testing.

This benchmark recognizes exact numeric -99 values as missing and uses
indicator-wise mean replacement to reproduce the published SmartPLS display.
The script performs only that bounded transformation on the 31 model indicators
and writes an auditable receipt alongside the cleaned CSV.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import pandas as pd


MODEL_COLUMNS = [
    "comp_1", "comp_2", "comp_3",
    "like_1", "like_2", "like_3",
    "cusl_1", "cusl_2", "cusl_3",
    "cusa",
    "csor_1", "csor_2", "csor_3", "csor_4", "csor_5",
    "attr_1", "attr_2", "attr_3",
    "perf_1", "perf_2", "perf_3", "perf_4", "perf_5",
    "qual_1", "qual_2", "qual_3", "qual_4", "qual_5", "qual_6", "qual_7", "qual_8",
]
EXPECTED_MISSING = {"cusl_1": 3, "cusl_2": 4, "cusl_3": 3, "cusa": 1}
EXPECTED_ROWS = 344
MISSING_MARKER = -99


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path, help="Corporate Reputation Data.xlsx")
    parser.add_argument("output", type=Path, help="Mean-replaced CSV output")
    parser.add_argument("receipt", type=Path, help="Preprocessing receipt JSON")
    parser.add_argument("--sheet", default="Sheet2")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    source = args.input.resolve()
    if not source.is_file():
        raise SystemExit(f"Input workbook does not exist: {source}")

    raw = pd.read_excel(source, sheet_name=args.sheet)
    if raw.columns.duplicated().any():
        raise SystemExit("Input workbook contains duplicate column names")
    missing_columns = [column for column in MODEL_COLUMNS if column not in raw.columns]
    if missing_columns:
        raise SystemExit(f"Input workbook is missing model columns: {missing_columns}")
    if len(raw) != EXPECTED_ROWS:
        raise SystemExit(f"Expected {EXPECTED_ROWS} observations, found {len(raw)}")
    if raw.duplicated().any():
        raise SystemExit("Input workbook contains duplicate observations")

    model = raw[MODEL_COLUMNS].apply(pd.to_numeric, errors="raise")
    missing_mask = model.eq(MISSING_MARKER)
    missing_counts = {
        column: int(count)
        for column, count in missing_mask.sum().items()
        if int(count) > 0
    }
    if missing_counts != EXPECTED_MISSING:
        raise SystemExit(
            f"Expected missing-marker counts {EXPECTED_MISSING}, found {missing_counts}"
        )

    recognized = model.mask(missing_mask)
    invalid = recognized.notna() & ((recognized < 1) | (recognized > 7))
    if invalid.any().any():
        locations = [
            {
                "excel_row": int(row) + 2,
                "column": MODEL_COLUMNS[column_index],
                "value": model.at[row, MODEL_COLUMNS[column_index]],
            }
            for row, column_index in zip(*invalid.to_numpy().nonzero(), strict=True)
        ]
        raise SystemExit(f"Unexpected model values outside 1..7: {locations}")

    replacement_means = recognized.mean(axis=0)
    cleaned = recognized.fillna(replacement_means)
    if cleaned.isna().any().any() or len(cleaned) != EXPECTED_ROWS:
        raise SystemExit("Mean replacement did not produce 344 complete observations")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    cleaned.to_csv(args.output, index=False, lineterminator="\n", float_format="%.15g")

    receipt = {
        "schema_version": 1,
        "benchmark": "smartpls_corporate_reputation",
        "source": {
            "file_name": source.name,
            "sha256": sha256_file(source),
            "sheet": args.sheet,
            "rows": int(len(raw)),
            "columns": int(len(raw.columns)),
            "all_cells_numeric": bool(
                raw.apply(lambda column: pd.to_numeric(column, errors="coerce").notna().all()).all()
            ),
            "duplicate_rows": int(raw.duplicated().sum()),
        },
        "selection": {
            "model_columns": MODEL_COLUMNS,
            "excluded_columns": [column for column in raw.columns if column not in MODEL_COLUMNS],
        },
        "cleaning": {
            "missing_marker": MISSING_MARKER,
            "missing_cells": int(missing_mask.sum().sum()),
            "rows_with_missing": int(missing_mask.any(axis=1).sum()),
            "missing_by_column": missing_counts,
            "treatment": "indicator_mean_replacement",
            "replacement_means": {
                column: float(replacement_means[column]) for column in missing_counts
            },
            "valid_response_range": [1, 7],
        },
        "output": {
            "file_name": args.output.name,
            "sha256": sha256_file(args.output),
            "rows": int(len(cleaned)),
            "columns": int(len(cleaned.columns)),
            "missing_cells": int(cleaned.isna().sum().sum()),
        },
    }
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
