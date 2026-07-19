"""Second-source R base-lm moderation reference for QuickPLS.

This development-only check mirrors the current single-item two-stage
moderation contract with R's built-in `lm`. It gives the experimental
`pls_two_stage_moderation_v1` slice a second executable source without adding
any R runtime requirement to QuickPLS itself.
"""

import json
import subprocess

import moderation_reference
from r_runtime import find_rscript


ROOT = moderation_reference.ROOT
RESULTS = moderation_reference.RESULTS
DATA = RESULTS / "moderation_r_reference.csv"
QUICKPLS = RESULTS / "moderation_r_reference_quickpls.json"
R_OUTPUT = RESULTS / "moderation_r_reference_values.json"
OUTPUT = RESULTS / "moderation_r_reference_report.json"
R_SCRIPT = ROOT / "validation" / "moderation_r_reference.R"
TOLERANCE = 1e-10


def quickpls_reference():
    rows = moderation_reference.generated_rows()
    result = moderation_reference.run_quickpls("moderation_r_reference", rows)
    result_path = RESULTS / "moderation_r_reference_quickpls.json"
    if result_path != QUICKPLS:
        raise AssertionError("unexpected moderation_r_reference result path")
    return result


def run_r_reference(rscript):
    subprocess.run(
        [
            rscript,
            "--vanilla",
            str(R_SCRIPT),
            str(DATA),
            str(R_OUTPUT),
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    data = json.loads(R_OUTPUT.read_text(encoding="utf-8"))
    paths = {}
    for key, value in data["paths"].items():
        source, target = key.split("->")
        paths[(source, target)] = value
    simple_slopes = {float(level): value for level, value in data["simple_slopes"].items()}
    return data, paths, simple_slopes


def compare_paths(reference, quickpls):
    differences = []
    for pair, expected in sorted(reference.items()):
        actual = quickpls["paths"][pair]
        differences.append(
            {
                "pair": list(pair),
                "expected": expected,
                "actual": actual,
                "abs_difference": abs(expected - actual),
            }
        )
    return differences


def compare_simple_slopes(reference, quickpls):
    actual = {
        row["moderator_score"]: row["effect"]
        for row in quickpls["moderation"][0]["simple_slopes"]
    }
    differences = []
    for level, expected in sorted(reference.items()):
        actual_value = actual[level]
        differences.append(
            {
                "moderator_score": level,
                "expected": expected,
                "actual": actual_value,
                "abs_difference": abs(expected - actual_value),
            }
        )
    return differences


def main():
    rscript, version = find_rscript()
    quickpls = quickpls_reference()
    r_payload, r_paths, r_simple_slopes = run_r_reference(rscript)
    path_differences = compare_paths(r_paths, quickpls)
    slope_differences = compare_simple_slopes(r_simple_slopes, quickpls)
    max_abs_difference = max(
        [row["abs_difference"] for row in path_differences + slope_differences]
    )
    passed = (
        max_abs_difference <= TOLERANCE
        and r_payload["used_observations"] == quickpls["used_observations"]
        and quickpls["moderation"][0]["interaction_effect"] == quickpls["paths"][("xm", "y")]
    )
    report = {
        "schema_version": 1,
        "kind": "pls_two_stage_moderation_r_lm_reference_v1",
        "runtime": {
            "command": "Rscript",
            "path": rscript,
            "version": version,
            "reference": r_payload["runtime"],
        },
        "source_data": str(DATA.relative_to(ROOT)).replace("\\", "/"),
        "quickpls_result": str(QUICKPLS.relative_to(ROOT)).replace("\\", "/"),
        "r_reference_output": str(R_OUTPUT.relative_to(ROOT)).replace("\\", "/"),
        "tolerance": TOLERANCE,
        "passed": passed,
        "max_abs_difference": max_abs_difference,
        "used_observations": {
            "quickpls": quickpls["used_observations"],
            "r": r_payload["used_observations"],
        },
        "path_differences": path_differences,
        "simple_slope_differences": slope_differences,
        "note": "Development-only second-source check using R base lm on the single-item two-stage moderation fixture.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"max_abs_difference={max_abs_difference:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
