import csv
import json
import subprocess
from pathlib import Path

from r_runtime import find_rscript

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "assessment_published_satisfaction.csv"
CSEM = RESULTS / "assessment_published_satisfaction_csem_0_6_1.csv"
RECIPE = RESULTS / "assessment_published_satisfaction.recipe.json"
QUICKPLS = RESULTS / "assessment_published_satisfaction_quickpls.json"
COMPARISON = RESULTS / "assessment_published_satisfaction_comparison.json"
TOLERANCE = 1e-6


def run_r_reference():
    rscript, _version = find_rscript()
    subprocess.run(
        [
            rscript,
            "--vanilla",
            "validation/assessment_published_fixture_csem.R",
            str(DATA),
            str(CSEM),
        ],
        cwd=ROOT,
        check=True,
    )


def cli_json(args):
    completed = subprocess.run(
        ["cargo", "run", "-p", "qpls-cli", "--", *args],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def dataset_fingerprint():
    project = RESULTS / "assessment_published_satisfaction.fingerprint.qpls"
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "import",
            str(DATA.relative_to(ROOT)),
            str(project.relative_to(ROOT)),
            "--name",
            "assessment_published_satisfaction",
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return cli_json(["inspect", str(project.relative_to(ROOT)), "--json"])["datasets"][0]["fingerprint"]


def write_recipe(fingerprint):
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000411",
        "created_at": "2026-07-18T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000412",
            "name": "cSEM satisfaction published assessment fixture",
            "constructs": [
                {"id": "IMAG", "name": "Image", "short_name": "IMAG", "mode": "formative", "indicators": ["imag1", "imag2", "imag3"]},
                {"id": "EXPE", "name": "Expectation", "short_name": "EXPE", "mode": "formative", "indicators": ["expe1", "expe2", "expe3"]},
                {"id": "QUAL", "name": "Quality", "short_name": "QUAL", "mode": "formative", "indicators": ["qual1", "qual2", "qual3", "qual4", "qual5"]},
                {"id": "VAL", "name": "Value", "short_name": "VAL", "mode": "formative", "indicators": ["val1", "val2", "val3"]},
                {"id": "SAT", "name": "Satisfaction", "short_name": "SAT", "mode": "reflective", "indicators": ["sat1", "sat2", "sat3", "sat4"]},
                {"id": "LOY", "name": "Loyalty", "short_name": "LOY", "mode": "reflective", "indicators": ["loy1", "loy2", "loy3", "loy4"]},
            ],
            "paths": [
                {"source": "IMAG", "target": "EXPE"},
                {"source": "EXPE", "target": "QUAL"},
                {"source": "EXPE", "target": "VAL"},
                {"source": "QUAL", "target": "VAL"},
                {"source": "IMAG", "target": "SAT"},
                {"source": "EXPE", "target": "SAT"},
                {"source": "QUAL", "target": "SAT"},
                {"source": "VAL", "target": "SAT"},
                {"source": "IMAG", "target": "LOY"},
                {"source": "SAT", "target": "LOY"},
            ],
        },
        "settings": {
            "method": "pls_pm",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260718,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "cSEM satisfaction",
            "source": "cSEM package README satisfaction example; GPL reference used only in validation tooling",
        },
    }
    RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


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
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))["payload"]["assessment"]


def load_csem():
    values = {}
    with CSEM.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            values[(row["metric"], row["target"], row["source"], row["variant"])] = float(row["value"])
    return values


def quickpls_values(assessment):
    values = {}
    for target, value in assessment["r_squared"].items():
        values[("r2", target, "", "csem_assess")] = float(value)
    for row in assessment["structural_quality"]:
        if row.get("adjusted_r_squared") is not None:
            values[("r2_adj", row["construct"], "", "csem_assess")] = float(row["adjusted_r_squared"])
    for row in assessment["structural_vif"]:
        values[("structural_vif", row["target_construct"], row["predictor_construct"], "csem_assess")] = float(row["vif"])
    for row in assessment["f_squared"]:
        if row.get("f_squared") is not None:
            values[("f_squared", row["target_construct"], row["source_construct"], "csem_assess_probe")] = float(row["f_squared"])
    fit = assessment["model_fit"]
    values[("srmr", "", "", "estimated")] = float(fit["estimated"]["srmr"])
    values[("d_uls", "", "", "estimated")] = float(fit["estimated"]["d_uls"])
    values[("srmr", "", "", "saturated")] = float(fit["saturated"]["srmr"])
    values[("d_uls", "", "", "saturated")] = float(fit["saturated"]["d_uls"])
    for row in assessment["construct_quality"]:
        if row.get("ave") is not None:
            values[("ave", row["construct"], "", "csem_assess")] = float(row["ave"])
        if row.get("rho_c") is not None:
            values[("rho_c", row["construct"], "", "csem_assess")] = float(row["rho_c"])
    constructs = assessment["fornell_larcker"]["constructs"]
    for row_index, row_construct in enumerate(constructs):
        for column_index, column_construct in enumerate(constructs):
            value = assessment["fornell_larcker"]["values"][row_index][column_index]
            if value is None:
                continue
            values[("fornell_larcker", row_construct, column_construct, "csem_assess")] = float(
                value
            )
    htmt_constructs = assessment["htmt_original"]["constructs"]
    for row_index, row_construct in enumerate(htmt_constructs):
        for column_index, column_construct in enumerate(htmt_constructs):
            cell = assessment["htmt_original"]["cells"][row_index][column_index]
            if cell["status"] == "available":
                values[("htmt_original", row_construct, column_construct, "csem_assess")] = float(cell["value"])
    return values


def compare(actual, reference):
    compared_keys = {
        key
        for key in reference
        if key[0]
        in {
            "r2",
            "r2_adj",
            "structural_vif",
            "f_squared",
            "srmr",
            "d_uls",
            "ave",
            "rho_c",
            "fornell_larcker",
            "htmt_original",
        }
    }
    rows = []
    for key in sorted(compared_keys):
        qpls = actual.get(key)
        csem = reference.get(key)
        difference = None if qpls is None or csem is None else qpls - csem
        abs_diff = None if difference is None else abs(difference)
        required = key not in {
            ("srmr", "", "", "estimated"),
            ("d_uls", "", "", "estimated"),
        }
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
    RESULTS.mkdir(parents=True, exist_ok=True)
    run_r_reference()
    write_recipe(dataset_fingerprint())
    quickpls = run_quickpls()
    comparisons = compare(quickpls_values(quickpls), load_csem())
    finite = [row["abs_diff"] for row in comparisons if row["abs_diff"] is not None]
    required_rows = [row for row in comparisons if row["required_for_status"]]
    optional_definition_rows = [
        row
        for row in comparisons
        if not row["required_for_status"] and row["classification"] == "definition_difference_probe"
    ]
    report = {
        "status": "passed" if all(row["passed"] for row in required_rows) else "failed",
        "tolerance": TOLERANCE,
        "reference": "cSEM 0.6.1 satisfaction README example using structured cSEM assessment helpers",
        "definition_mapping": {
            "required": "R2, adjusted R2, structural VIF, fixed-score Cohen f-squared, AVE, rho_C, Fornell-Larcker, original HTMT, and saturated SRMR/d_ULS are treated as equivalent definitions for the cSEM satisfaction README fixture.",
            "optional": "Estimated SRMR/d_ULS are retained as mixed composite/common-factor definition probes; the corporate-reputation fixture supplies required estimated-fit agreement.",
        },
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "csem": str(CSEM.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
        },
        "metric_count": len(comparisons),
        "required_metric_count": len(required_rows),
        "optional_definition_difference_count": len(optional_definition_rows),
        "max_abs_diff": max(finite) if finite else None,
        "required_max_abs_diff": max(
            row["abs_diff"] for row in required_rows if row["abs_diff"] is not None
        )
        if required_rows
        else None,
        "comparisons": comparisons,
    }
    COMPARISON.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"published assessment fixture comparison failed; see {COMPARISON}")


if __name__ == "__main__":
    main()
