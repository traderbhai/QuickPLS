"""Published/example mediation fixture for QuickPLS.

This check reuses the documented cSEM `threecommonfactors` example already used
for v0.3 PLS agreement. The existing cSEM fixture verifies the path coefficients
against an external engine; this script then independently recomputes direct,
indirect, total, VAF, and descriptive mediation classes from the validated path
matrix and compares them to `pls_mediation_v1`.
"""

import json
import math
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
PLS_COMPARISON = RESULTS / "pls_csem_threecommonfactors_comparison.json"
QUICKPLS = RESULTS / "pls_quickpls_csem_threecommonfactors.json"
OUTPUT = RESULTS / "mediation_published_example_report.json"
TOLERANCE = 1e-12
PATH_TOLERANCE = 1e-6


def run_published_pls_fixture():
    subprocess.run(
        [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "validation/run_pls_published_csem.ps1",
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )


def load_inputs():
    comparison = json.loads(PLS_COMPARISON.read_text(encoding="utf-8"))
    quickpls = json.loads(QUICKPLS.read_text(encoding="utf-8"))
    if comparison.get("status") != "passed":
        raise SystemExit(f"published PLS fixture did not pass; see {PLS_COMPARISON}")
    if comparison.get("max_abs_diff", math.inf) > PATH_TOLERANCE:
        raise SystemExit(f"published PLS path agreement exceeded tolerance; see {PLS_COMPARISON}")
    estimation = quickpls["payload"]["estimation"]
    mediation = estimation.get("mediation")
    if not mediation:
        raise SystemExit("QuickPLS published fixture result does not contain mediation output")
    return comparison, estimation, mediation


def build_construct_order(paths):
    order = []
    for path in paths:
        for construct in [path["source"], path["target"]]:
            if construct not in order:
                order.append(construct)
    return order


def independent_effect_decomposition(paths):
    constructs = build_construct_order(paths)
    index = {name: offset for offset, name in enumerate(constructs)}
    size = len(constructs)
    direct = [[0.0 for _ in constructs] for _ in constructs]
    for path in paths:
        direct[index[path["source"]]][index[path["target"]]] = float(path["coefficient"])

    total = [[direct[row][col] for col in range(size)] for row in range(size)]
    power = [[direct[row][col] for col in range(size)] for row in range(size)]
    for _ in range(2, size + 1):
        power = matrix_multiply(power, direct)
        for row in range(size):
            for col in range(size):
                total[row][col] += power[row][col]

    effects = {}
    for source in constructs:
        for target in constructs:
            if source == target:
                continue
            row = index[source]
            col = index[target]
            direct_value = direct[row][col]
            total_value = total[row][col]
            indirect_value = total_value - direct_value
            if abs(direct_value) <= TOLERANCE and abs(indirect_value) <= TOLERANCE:
                continue
            effects[(source, target)] = {
                "direct": direct_value,
                "indirect": indirect_value,
                "total": total_value,
                "variance_accounted_for": variance_accounted_for(indirect_value, total_value),
                "mediation_class": mediation_class(direct_value, indirect_value),
            }
    return effects


def matrix_multiply(left, right):
    size = len(left)
    return [
        [sum(left[row][k] * right[k][col] for k in range(size)) for col in range(size)]
        for row in range(size)
    ]


def variance_accounted_for(indirect, total):
    if abs(total) <= TOLERANCE:
        return None
    return indirect / total


def mediation_class(direct, indirect):
    has_direct = abs(direct) > TOLERANCE
    has_indirect = abs(indirect) > TOLERANCE
    if not has_direct and not has_indirect:
        return "no_effect"
    if has_direct and not has_indirect:
        return "direct_only"
    if not has_direct and has_indirect:
        return "indirect_only"
    if direct * indirect > 0.0:
        return "complementary_partial"
    return "competitive_partial"


def compare(reference, mediation):
    actual_by_pair = {
        (row["source"], row["target"]): row for row in mediation["estimates"]
    }
    differences = []
    for pair, expected in sorted(reference.items()):
        actual = actual_by_pair[pair]
        for metric in ["direct", "indirect", "total", "variance_accounted_for"]:
            expected_value = expected[metric]
            actual_value = actual[metric]
            if expected_value is None and actual_value is None:
                abs_difference = 0.0
            else:
                abs_difference = abs(expected_value - actual_value)
            differences.append(
                {
                    "pair": list(pair),
                    "metric": metric,
                    "expected": expected_value,
                    "actual": actual_value,
                    "abs_difference": abs_difference,
                }
            )
        differences.append(
            {
                "pair": list(pair),
                "metric": "mediation_class",
                "expected": expected["mediation_class"],
                "actual": actual["classification"],
                "passed": expected["mediation_class"] == actual["classification"],
            }
        )
    numeric_max = max(
        row.get("abs_difference", 0.0) for row in differences if "abs_difference" in row
    )
    classes_pass = all(row.get("passed", True) for row in differences)
    nonzero_indirect_pairs = [
        list(pair) for pair, row in reference.items() if abs(row["indirect"]) > TOLERANCE
    ]
    return differences, numeric_max, classes_pass, nonzero_indirect_pairs


def main():
    run_published_pls_fixture()
    comparison, estimation, mediation = load_inputs()
    reference = independent_effect_decomposition(estimation["paths"])
    differences, max_abs_difference, classes_pass, nonzero_indirect_pairs = compare(
        reference, mediation
    )
    passed = (
        max_abs_difference <= TOLERANCE
        and classes_pass
        and len(nonzero_indirect_pairs) > 0
        and mediation.get("method_version") == "pls_mediation_v1"
    )
    report = {
        "schema_version": 1,
        "kind": "pls_mediation_published_example_v1",
        "method_version": mediation.get("method_version"),
        "source": {
            "dataset": comparison["source"]["dataset"],
            "documentation": comparison["source"]["documentation"],
            "population_path_values": comparison["source"]["population_path_values"],
            "path_agreement_artifact": str(PLS_COMPARISON.relative_to(ROOT)).replace("\\", "/"),
            "quickpls_result": str(QUICKPLS.relative_to(ROOT)).replace("\\", "/"),
        },
        "tolerance": TOLERANCE,
        "path_tolerance": PATH_TOLERANCE,
        "path_fixture_max_abs_diff": comparison["max_abs_diff"],
        "passed": passed,
        "max_abs_difference": max_abs_difference,
        "classes_pass": classes_pass,
        "nonzero_indirect_pairs": nonzero_indirect_pairs,
        "differences": differences,
        "note": "Documented cSEM example fixture; effects are independently decomposed from path coefficients already verified against cSEM.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"max_abs_difference={max_abs_difference:.3g} | "
        f"nonzero_indirect_pairs={nonzero_indirect_pairs}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
