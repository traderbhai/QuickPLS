import csv
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "corporate_reputation.csv"
RECIPE = ROOT / "validation" / "fixtures" / "corporate_reputation.recipe.json"
CSEM = ROOT / "validation" / "results" / "assessment_csem_0_6_1.csv"
QUICKPLS = ROOT / "validation" / "results" / "assessment_quickpls_reference.json"
OUTPUT = ROOT / "validation" / "results" / "assessment_csem_comparison.json"
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
    payload = envelope["payload"]
    return payload["assessment"]


def load_csem():
    values = {}
    with CSEM.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            key = (row["metric"], row["target"], row["source"], row["variant"])
            values[key] = float(row["value"])
    return values


def quickpls_values(assessment):
    values = {}
    for target, value in assessment["r_squared"].items():
        values[("r2", target, "", "csem_assess")] = float(value)
    for row in assessment["structural_quality"]:
        if row.get("adjusted_r_squared") is not None:
            values[("r2_adj", row["construct"], "", "csem_assess")] = float(
                row["adjusted_r_squared"]
            )
    for row in assessment["structural_vif"]:
        values[("structural_vif", row["target_construct"], row["predictor_construct"], "csem_assess")] = float(
            row["vif"]
        )
    for row in assessment["f_squared"]:
        if row.get("f_squared") is not None:
            values[("f_squared", row["target_construct"], row["source_construct"], "csem_assess_probe")] = float(
                row["f_squared"]
            )
    fit = assessment["model_fit"]
    for variant in ("estimated", "saturated"):
        values[("srmr", "", "", variant)] = float(fit[variant]["srmr"])
        values[("d_uls", "", "", variant)] = float(fit[variant]["d_uls"])
    return values


def compare(required_keys, optional_keys, actual, reference):
    rows = []
    for key in sorted(required_keys | optional_keys):
        qpls = actual.get(key)
        csem = reference.get(key)
        difference = None if qpls is None or csem is None else qpls - csem
        abs_diff = None if difference is None else abs(difference)
        required = key in required_keys
        passed = abs_diff is not None and abs_diff <= TOLERANCE
        rows.append(
            {
                "metric": key[0],
                "target": key[1],
                "source": key[2],
                "variant": key[3],
                "quickpls": qpls,
                "csem": csem,
                "difference": difference,
                "abs_diff": abs_diff,
                "passed": passed,
                "required_for_status": required,
                "classification": "equivalent"
                if passed
                else ("definition_difference_probe" if not required else "failed_required_comparison"),
            }
        )
    return rows


def main():
    assessment = run_quickpls()
    actual = quickpls_values(assessment)
    reference = load_csem()
    required_keys = {
        key
        for key in reference
        if key[0] in {"r2", "r2_adj", "structural_vif", "f_squared", "srmr", "d_uls"}
    }
    optional_keys = set()
    comparisons = compare(required_keys, optional_keys, actual, reference)
    required_rows = [row for row in comparisons if row["required_for_status"]]
    finite_required = [row["abs_diff"] for row in required_rows if row["abs_diff"] is not None]
    optional_definition_rows = [
        row
        for row in comparisons
        if not row["required_for_status"] and row["classification"] == "definition_difference_probe"
    ]
    report = {
        "status": "passed" if all(row["passed"] for row in required_rows) else "failed",
        "tolerance": TOLERANCE,
        "reference": "cSEM 0.6.1 assess(), calculateDL(), and calculateSRMR()",
        "definition_mapping": {
            "required": "R2, adjusted R2, structural VIF rows emitted by cSEM, fixed-score Cohen f-squared, and estimated/saturated SRMR/d_ULS are treated as equivalent definitions.",
            "f_squared": "QuickPLS removes one structural predictor at a time and recomputes the target equation using the already-estimated construct scores, matching cSEM calculatef2().",
        },
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "csem": str(CSEM.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
        },
        "required_metric_count": len(required_rows),
        "optional_probe_count": len(optional_keys),
        "optional_definition_difference_count": len(optional_definition_rows),
        "required_max_abs_diff": max(finite_required) if finite_required else None,
        "comparisons": comparisons,
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"assessment cSEM comparison failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
