"""Publication audit for v0.2 data and project platform."""

import csv
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "data_project_publication_audit.json"
CLI = ROOT / "target" / "debug" / "qpls.exe"


def run(command, expect_success=True, timeout=240):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    passed = proc.returncode == 0 if expect_success else proc.returncode != 0
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": passed,
        "stdout_tail": proc.stdout[-3000:],
        "stderr_tail": proc.stderr[-3000:],
    }


def ensure_cli():
    build = run(["cargo", "build", "-p", "qpls-cli"])
    if not build["passed"]:
        raise RuntimeError(build)
    return build


def write_csv(path, rows, cols):
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(cols)
        writer.writerows(rows)


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    build = ensure_cli()
    wide = RESULTS / "publication_data_wide.csv"
    long = RESULTS / "publication_data_long.csv"
    missing = RESULTS / "publication_data_missing.csv"
    malformed = RESULTS / "publication_data_duplicate_headers.csv"
    tsv = RESULTS / "publication_data.tsv"
    covariance = RESULTS / "publication_covariance.csv"
    write_csv(wide, [[i + j * 0.01 for j in range(40)] for i in range(50)], [f"v{j}" for j in range(40)])
    write_csv(long, [[i, i * 0.5, i % 7] for i in range(2000)], ["x", "y", "group"])
    write_csv(missing, [[i if i % 5 else "", i * 0.25 if i % 7 else "NA"] for i in range(200)], ["x", "y"])
    write_csv(malformed, [[1, 2], [3, 4]], ["dup", "dup"])
    tsv.write_text("a\tb\tc\n1\t2\t3\n4\t5\t6\n", encoding="utf-8")
    write_csv(covariance, [[1, 0.3, 0.2], [0.3, 1, 0.4], [0.2, 0.4, 1]], ["a", "b", "c"])
    imports = [
        run([str(CLI), "import", str(wide.relative_to(ROOT)), str((RESULTS / "publication_data_wide.qpls").relative_to(ROOT)), "--name", "wide"]),
        run([str(CLI), "import", str(long.relative_to(ROOT)), str((RESULTS / "publication_data_long.qpls").relative_to(ROOT)), "--name", "long"]),
        run([str(CLI), "import", str(missing.relative_to(ROOT)), str((RESULTS / "publication_data_missing.qpls").relative_to(ROOT)), "--name", "missing"]),
        run([str(CLI), "import", str(tsv.relative_to(ROOT)), str((RESULTS / "publication_data_tsv.qpls").relative_to(ROOT)), "--name", "tsv", "--delimiter", "\t"]),
        run([str(CLI), "import", str(covariance.relative_to(ROOT)), str((RESULTS / "publication_covariance.qpls").relative_to(ROOT)), "--name", "covariance", "--kind", "covariance", "--sample-size", "200"]),
        run([str(CLI), "import", str(malformed.relative_to(ROOT)), str((RESULTS / "publication_data_malformed.qpls").relative_to(ROOT)), "--name", "malformed"], expect_success=False),
    ]
    cargo_checks = [
        run(["cargo", "test", "-p", "qpls-data"]),
        run(["cargo", "test", "-p", "qpls-project"]),
    ]
    combined_output = "\n".join(item["stdout_tail"] + item["stderr_tail"] for item in cargo_checks)
    required_tests = [
        "duplicate_headers_are_rejected",
        "csv_import_infers_types_missing_values_and_round_trips_arrow",
        "correlation_matrix_requires_square_symmetric_unit_diagonal_data",
        "sav_fixture_preserves_rows_columns_and_measure_metadata",
        "xlsx_fixture_imports_numeric_and_text_columns",
        "project_round_trip_preserves_arrow_dataset_and_manifest",
        "truncated_archive_is_rejected",
        "previous_generation_recovers_a_corrupt_primary_archive",
        "version_one_archive_migrates_to_the_current_schema",
        "valid_autosave_takes_precedence_and_can_be_discarded",
    ]
    test_coverage = {name: name in combined_output for name in required_tests}
    passed = build["passed"] and all(item["passed"] for item in imports + cargo_checks) and all(test_coverage.values())
    report = {
        "schema_version": 1,
        "target": "v0.2 data/project publication audit",
        "passed": passed,
        "generated_fixtures": [str(path.relative_to(ROOT)) for path in [wide, long, missing, malformed, tsv, covariance]],
        "import_checks": imports,
        "cargo_checks": cargo_checks,
        "test_coverage": test_coverage,
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
