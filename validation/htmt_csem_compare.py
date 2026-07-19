import csv
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "corporate_reputation.csv"
RECIPE = ROOT / "validation" / "fixtures" / "corporate_reputation.recipe.json"
CSEM = ROOT / "validation" / "results" / "htmt_csem_0_6_1.csv"
QUICKPLS = ROOT / "validation" / "results" / "htmt_quickpls_reference.json"
OUTPUT = ROOT / "validation" / "results" / "htmt_csem_comparison.json"
TOLERANCE = 1e-6


def run_quickpls():
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
        ],
        cwd=ROOT,
        check=True,
    )
    envelope = json.loads(QUICKPLS.read_text(encoding="utf-8"))
    assessment = envelope["payload"]["assessment"]
    return {
        "htmt_plus": assessment["htmt_plus"],
        "htmt_original": assessment["htmt_original"],
    }


def load_csem():
    values = {}
    with CSEM.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            values[(row["variant"], row["row"], row["column"])] = float(row["value"])
    return values


def compare_matrix(variant, artifact, csem, require_equivalence):
    constructs = artifact["constructs"]
    comparisons = []
    for row_index, row_name in enumerate(constructs):
        for column_index, column_name in enumerate(constructs):
            cell = artifact["cells"][row_index][column_index]
            reference = csem[(variant, row_name, column_name)]
            actual = cell["value"]
            if actual is None:
                passed = False
                difference = None
                abs_diff = None
            else:
                actual = float(actual)
                difference = actual - reference
                abs_diff = abs(difference)
                passed = abs_diff <= TOLERANCE
            comparisons.append(
                {
                    "variant": variant,
                    "row": row_name,
                    "column": column_name,
                    "quickpls_status": cell["status"],
                    "quickpls": actual,
                    "csem": reference,
                    "difference": difference,
                    "abs_diff": abs_diff,
                    "passed": passed,
                    "required_for_status": require_equivalence,
                    "classification": "equivalent"
                    if passed
                    else (
                        "definition_difference"
                        if not require_equivalence
                        else "failed_required_comparison"
                    ),
                }
            )
    return comparisons


def main():
    quickpls = run_quickpls()
    csem = load_csem()
    comparisons = []
    comparisons.extend(compare_matrix("htmt_plus", quickpls["htmt_plus"], csem, False))
    comparisons.extend(compare_matrix("htmt_original", quickpls["htmt_original"], csem, True))
    finite_diffs = [row["abs_diff"] for row in comparisons if row["abs_diff"] is not None]
    required = [row for row in comparisons if row["required_for_status"]]
    optional_definition_differences = [
        row
        for row in comparisons
        if not row["required_for_status"] and row["classification"] == "definition_difference"
    ]
    report = {
        "status": "passed" if all(row["passed"] for row in required) else "failed",
        "tolerance": TOLERANCE,
        "reference": "cSEM 0.6.1 calculateHTMT(.type_htmt='htmt')",
        "definition_mapping": {
            "htmt_plus": "QuickPLS HTMT+ is not equivalent to cSEM .absolute=TRUE when signed cross-block correlations differ; nonmatching cells are recorded as definition differences.",
            "htmt_original": "calculateHTMT(..., .absolute = FALSE)",
        },
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "csem": str(CSEM.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
        },
        "compared_quantities": ["original signed HTMT matrix"],
        "non_equivalent_probe_quantities": ["cSEM .absolute=TRUE versus QuickPLS HTMT+"],
        "max_abs_diff": max(finite_diffs),
        "required_max_abs_diff": max(row["abs_diff"] for row in required if row["abs_diff"] is not None),
        "optional_definition_difference_count": len(optional_definition_differences),
        "comparisons": comparisons,
        "note": "cSEM stores one triangle and zeros in the opposite triangle; the R runner symmetrizes before comparison. Required status is based on original signed HTMT. HTMT+ remains governed by the frozen Ringle et al. mean-absolute-correlation specification and still needs an equivalent external engine or rounded appendix fixture.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"HTMT cSEM comparison failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
