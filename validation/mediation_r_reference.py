"""Second-source R base-lm mediation reference for QuickPLS.

This development-only check uses R's built-in `lm` on sample-standardized
single-item variables. It provides a second executable reference source for the
current descriptive `pls_mediation_v1` fixture without adding any R runtime
requirement to QuickPLS itself.
"""

import json
import subprocess
from pathlib import Path

import mediation_reference
from r_runtime import find_rscript


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
R_OUTPUT = RESULTS / "mediation_r_reference_values.json"
OUTPUT = RESULTS / "mediation_r_reference_report.json"
R_SCRIPT = ROOT / "validation" / "mediation_r_reference.R"
TOLERANCE = 1e-12


def quickpls_mediation_rows():
    mediation_reference.main()
    result = json.loads(mediation_reference.QUICKPLS.read_text(encoding="utf-8"))
    rows = result["payload"]["estimation"]["mediation"]["estimates"]
    return {(row["source"], row["target"]): row for row in rows}


def run_r_reference(rscript):
    subprocess.run(
        [
            rscript,
            "--vanilla",
            str(R_SCRIPT),
            str(mediation_reference.DATA),
            str(R_OUTPUT),
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    data = json.loads(R_OUTPUT.read_text(encoding="utf-8"))
    reference = {}
    for key, value in data["mediation"].items():
        source, target = key.split("->")
        reference[(source, target)] = value
    return data, reference


def compare(reference, quickpls):
    differences = []
    for pair, expected in reference.items():
        actual = quickpls[pair]
        for metric in ["direct", "indirect", "total", "variance_accounted_for"]:
            actual_value = actual[metric]
            expected_value = expected[metric]
            differences.append(
                {
                    "pair": list(pair),
                    "metric": metric,
                    "expected": expected_value,
                    "actual": actual_value,
                    "abs_difference": abs(expected_value - actual_value),
                }
            )
    return differences, max(row["abs_difference"] for row in differences)


def main():
    rscript, version = find_rscript()
    quickpls = quickpls_mediation_rows()
    r_payload, reference = run_r_reference(rscript)
    differences, max_abs_difference = compare(reference, quickpls)
    report = {
        "schema_version": 1,
        "kind": "pls_mediation_r_lm_reference_v1",
        "runtime": {
            "command": "Rscript",
            "path": rscript,
            "version": version,
            "reference": r_payload["runtime"],
        },
        "source_data": str(mediation_reference.DATA.relative_to(ROOT)).replace("\\", "/"),
        "quickpls_result": str(mediation_reference.QUICKPLS.relative_to(ROOT)).replace("\\", "/"),
        "r_reference_output": str(R_OUTPUT.relative_to(ROOT)).replace("\\", "/"),
        "tolerance": TOLERANCE,
        "passed": max_abs_difference <= TOLERANCE,
        "max_abs_difference": max_abs_difference,
        "differences": differences,
        "note": "Development-only second-source check using R base lm on the single-item standardized mediation fixture.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={report['passed']} | "
        f"max_abs_difference={max_abs_difference:.3g}"
    )
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
