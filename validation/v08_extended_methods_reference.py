"""Integrated v0.8 extended-method validation fixtures.

These checks are preview gates, not publication validation. They prove that the
experimental v0.8 product boundary emits typed payloads and matches independent
NumPy/Python calculations for bounded fixtures.
"""

import argparse
import csv
import json
import os
import math
import random
import subprocess
from pathlib import Path

import numpy as np


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "v08_extended_methods_fixture.csv"
OUTPUT = RESULTS / "v08_extended_methods_reference_report.json"
CONFIGURED_CLI = os.environ.get("QUICKPLS_CLI_PATH", "").strip()
CLI_EXE = Path(CONFIGURED_CLI).resolve() if CONFIGURED_CLI else ROOT / "target" / "debug" / "qpls.exe"
CLI_READY = False
TOL = 1e-6


def ensure_cli():
    global CLI_READY
    if not CLI_READY:
        if not CLI_EXE.exists():
            if CONFIGURED_CLI:
                raise FileNotFoundError(f"QUICKPLS_CLI_PATH does not exist: {CLI_EXE}")
            subprocess.run(["cargo", "build", "-p", "qpls-cli"], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
        CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def write_dataset():
    RESULTS.mkdir(parents=True, exist_ok=True)
    rng = random.Random(20260719)
    fields = ["x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"]
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for index in range(1, 141):
            x = rng.gauss(0.0, 1.0)
            w = rng.gauss(0.0, 1.0)
            z = rng.gauss(0.0, 1.0)
            m = 0.62 * x + 0.22 * z + rng.gauss(0.0, 0.45)
            y = 0.30 * x + 0.68 * m + 0.24 * w + 0.18 * x * w + rng.gauss(0.0, 0.42)
            logit = -0.15 + 1.05 * x - 0.55 * z + 0.35 * w
            p = 1.0 / (1.0 + math.exp(-logit))
            bin_y = 1 if rng.random() < p else 0
            writer.writerow({
                "x": f"{x:.12f}",
                "m": f"{m:.12f}",
                "w": f"{w:.12f}",
                "y": f"{y:.12f}",
                "z": f"{z:.12f}",
                "bin_y": str(bin_y),
                "g1": f"{x + rng.gauss(0.0, 0.08):.12f}",
                "g2": f"{0.86 * x + rng.gauss(0.0, 0.10):.12f}",
                "g3": f"{0.74 * x + rng.gauss(0.0, 0.12):.12f}",
                "h1": f"{y + rng.gauss(0.0, 0.08):.12f}",
                "h2": f"{0.88 * y + rng.gauss(0.0, 0.10):.12f}",
            })


def rows():
    with DATA.open(newline="", encoding="utf-8") as handle:
        return [{key: float(value) for key, value in row.items()} for row in csv.DictReader(handle)]


def dataset_fingerprint(name):
    project_path = RESULTS / f"{name}.fingerprint.qpls"
    qpls(["import", str(DATA.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", name], check=True, stdout=subprocess.DEVNULL)
    payload = json.loads(qpls(["inspect", str(project_path.relative_to(ROOT)), "--json"], check=True, capture_output=True, text=True).stdout)
    return payload["datasets"][0]["fingerprint"]


def base_settings(method):
    return {
        "method": method,
        "weighting_scheme": "path",
        "tolerance": 1e-9,
        "max_iterations": 3000,
        "bootstrap_samples": 0,
        "seed": 20260719,
        "preprocessing": "unstandardized" if method == "nca" else "standardized",
        "missing_data": "listwise_deletion",
    }


def empty_model():
    return {
        "id": "00000000-0000-0000-0000-000000008001",
        "name": "v0.8 standalone method validation",
        "constructs": [],
        "paths": [],
    }


def gsca_model():
    return {
        "id": "00000000-0000-0000-0000-000000008002",
        "name": "v0.8 GSCA validation",
        "constructs": [
            {"id": "g", "name": "G", "short_name": "G", "mode": "reflective", "indicators": ["g1", "g2", "g3"]},
            {"id": "h", "name": "H", "short_name": "H", "mode": "reflective", "indicators": ["h1", "h2"]},
        ],
        "paths": [{"source": "g", "target": "h"}],
    }


def run_recipe(name, method, metadata, model=None):
    fingerprint = dataset_fingerprint(name)
    recipe = RESULTS / f"{name}.recipe.json"
    output = RESULTS / f"{name}_quickpls.json"
    payload = {
        "schema_version": 2,
        "id": f"00000000-0000-0000-0000-00000008{sum(ord(ch) for ch in name) % 10000:04d}",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": model or empty_model(),
        "settings": base_settings(method),
        "metadata": {"fixture": "v08_extended_methods_reference", **metadata},
    }
    recipe.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    qpls(["run", str(recipe.relative_to(ROOT)), "--data", str(DATA.relative_to(ROOT)), "--output", str(output.relative_to(ROOT)), "--allow-experimental"], check=True, stdout=subprocess.DEVNULL)
    return json.loads(output.read_text(encoding="utf-8"))["payload"]["estimation"]


def matrix(columns):
    data = rows()
    return np.array([[row[column] for column in columns] for row in data], dtype=float)


def standardize(values):
    return (values - values.mean(axis=0)) / values.std(axis=0, ddof=1)


def ols_fit(y, predictors):
    x = np.column_stack([np.ones(len(y)), predictors])
    beta = np.linalg.lstsq(x, y, rcond=None)[0]
    fitted = x @ beta
    resid = y - fitted
    ssr = float(np.sum(resid ** 2))
    sst = float(np.sum((y - y.mean()) ** 2))
    return beta, 1.0 - ssr / sst


def check_pca():
    estimation = run_recipe("v08_pca", "pca", {
        "pca_variables": "x,m,w,y,z",
        "pca_component_rule": "fixed",
        "pca_components": "3",
    })
    pca = estimation["pca"]
    z = standardize(matrix(["x", "m", "w", "y", "z"]))
    corr = np.cov(z, rowvar=False, ddof=1)
    vals = np.linalg.eigh(corr)[0][::-1]
    diffs = [abs(pca["components"][i]["eigenvalue"] - float(vals[i])) for i in range(3)]
    return {
        "passed": max(diffs) <= TOL,
        "method_version": pca["method_version"],
        "max_abs_difference": max(diffs),
        "retained_components": pca["retained_components"],
    }


def check_ols():
    estimation = run_recipe("v08_regression_ols", "regression", {
        "regression_type": "ols",
        "regression_outcome": "y",
        "regression_predictors": "x,m",
        "regression_controls": "z",
        "robust_se": "hc3",
    })
    reg = estimation["regression"]
    data = matrix(["y", "x", "m", "z"])
    beta, r2 = ols_fit(data[:, 0], data[:, 1:])
    by_term = {row["term"]: row["estimate"] for row in reg["coefficients"]}
    diffs = [
        abs(by_term["intercept"] - float(beta[0])),
        abs(by_term["x"] - float(beta[1])),
        abs(by_term["m"] - float(beta[2])),
        abs(by_term["z"] - float(beta[3])),
        abs(reg["fit"]["r_squared"] - float(r2)),
    ]
    return {"passed": max(diffs) <= TOL, "method_version": reg["method_version"], "max_abs_difference": max(diffs)}


def logistic_reference(y, predictors):
    x = np.column_stack([np.ones(len(y)), predictors])
    beta = np.zeros(x.shape[1])
    for _ in range(80):
        eta = x @ beta
        p = 1.0 / (1.0 + np.exp(-np.clip(eta, -30, 30)))
        w = np.maximum(p * (1.0 - p), 1e-9)
        z = eta + (y - p) / w
        xtw = x.T * w
        next_beta = np.linalg.solve(xtw @ x, xtw @ z)
        if np.max(np.abs(next_beta - beta)) < 1e-10:
            beta = next_beta
            break
        beta = next_beta
    return beta


def check_logistic():
    estimation = run_recipe("v08_regression_logistic", "regression", {
        "regression_type": "logistic",
        "regression_outcome": "bin_y",
        "regression_predictors": "x,z,w",
    })
    reg = estimation["regression"]
    data = matrix(["bin_y", "x", "z", "w"])
    beta = logistic_reference(data[:, 0], data[:, 1:])
    by_term = {row["term"]: row["estimate"] for row in reg["coefficients"]}
    diffs = [
        abs(by_term["intercept"] - float(beta[0])),
        abs(by_term["x"] - float(beta[1])),
        abs(by_term["z"] - float(beta[2])),
        abs(by_term["w"] - float(beta[3])),
    ]
    return {"passed": max(diffs) <= 1e-5, "method_version": reg["method_version"], "max_abs_difference": max(diffs)}


def check_process():
    estimation = run_recipe("v08_process", "regression", {
        "regression_type": "process",
        "regression_outcome": "y",
        "regression_predictors": "x,m",
        "process_model": "mediation",
        "process_x": "x",
        "process_m": "m",
    })
    reg = estimation["regression"]
    process = reg["process"]
    data = matrix(["x", "m", "y"])
    a, _ = ols_fit(data[:, 1], data[:, [0]])
    b_model, _ = ols_fit(data[:, 2], data[:, [0, 1]])
    indirect = float(a[1] * b_model[2])
    reported = {row["effect"]: row["estimate"] for row in process["effects"]}
    diff = abs(reported["indirect"] - indirect)
    return {"passed": diff <= TOL, "method_version": process["method_version"], "max_abs_difference": diff}


def nca_reference(x, y):
    """Independent CE-FDH peer, CR-FDH regression, and bottleneck equations.

    CE-FDH is expressed as cumulative record highs over unique ascending X.
    CR-FDH is a separate least-squares fit through those peers. This avoids
    repeating the production implementation's iteration structure.
    """
    maximum_y_by_x = {}
    for x_value, y_value in zip(x, y):
        x_value = float(x_value)
        maximum_y_by_x[x_value] = max(maximum_y_by_x.get(x_value, -math.inf), float(y_value))
    peers = []
    record_high = -math.inf
    for x_value, y_value in sorted(maximum_y_by_x.items()):
        if y_value > record_high:
            peers.append((x_value, y_value))
            record_high = y_value

    minimum_x, maximum_x = float(np.min(x)), float(np.max(x))
    minimum_y, maximum_y = float(np.min(y)), float(np.max(y))
    scope_area = (maximum_x - minimum_x) * (maximum_y - minimum_y)
    ce_area = sum(
        ((peers[index + 1][0] if index + 1 < len(peers) else maximum_x) - peer[0])
        * (maximum_y - peer[1])
        for index, peer in enumerate(peers)
    )
    ce_effect = ce_area / scope_area

    design = np.column_stack([np.ones(len(peers)), np.array([peer[0] for peer in peers])])
    intercept, slope = np.linalg.lstsq(design, np.array([peer[1] for peer in peers]), rcond=None)[0]
    breakpoints = [minimum_x, maximum_x]
    for boundary_y in (minimum_y, maximum_y):
        crossing = (boundary_y - float(intercept)) / float(slope)
        if minimum_x < crossing < maximum_x:
            breakpoints.append(crossing)
    breakpoints = sorted(set(breakpoints))
    cr_area = 0.0
    for left, right in zip(breakpoints, breakpoints[1:]):
        left_y = min(maximum_y, max(minimum_y, float(intercept + slope * left)))
        right_y = min(maximum_y, max(minimum_y, float(intercept + slope * right)))
        cr_area += (right - left) * ((maximum_y - left_y) + (maximum_y - right_y)) / 2.0

    def bottleneck(ceiling, outcome_percent):
        threshold = minimum_y + (maximum_y - minimum_y) * outcome_percent / 100.0
        required = None
        status = "not_attainable"
        if ceiling == "ce_fdh":
            if threshold <= peers[0][1]:
                status = "not_necessary"
            else:
                required_peer = next((peer for peer in peers if peer[1] >= threshold), None)
                if required_peer is not None:
                    required, status = required_peer[0], "required"
        else:
            left_y = float(intercept + slope * minimum_x)
            right_y = float(intercept + slope * maximum_x)
            if threshold <= left_y:
                status = "not_necessary"
            elif threshold <= right_y:
                required, status = float((threshold - intercept) / slope), "required"
        required_percent = None if required is None else 100.0 * (required - minimum_x) / (maximum_x - minimum_x)
        return {
            "ceiling": ceiling,
            "outcome_percent": float(outcome_percent),
            "required_x_percent": required_percent,
            "status": status,
        }

    return {
        "scope": {"minimum_x": minimum_x, "maximum_x": maximum_x, "minimum_y": minimum_y, "maximum_y": maximum_y},
        "peers": peers,
        "ce_effect": ce_effect,
        "cr_effect": cr_area / scope_area,
        "slope": float(slope),
        "intercept": float(intercept),
        "bottlenecks": [bottleneck(ceiling, level) for ceiling in ("ce_fdh", "cr_fdh") for level in range(10, 100, 10)],
    }


def check_nca():
    estimation = run_recipe("v08_nca", "nca", {
        "nca_x": "x",
        "nca_y": "y",
        "nca_ceiling": "both",
        "nca_permutation_samples": "99",
    })
    repeated = run_recipe("v08_nca_repeat", "nca", {
        "nca_x": "x",
        "nca_y": "y",
        "nca_ceiling": "both",
        "nca_permutation_samples": "99",
    })["nca"]
    nca = estimation["nca"]
    envelope = json.loads((RESULTS / "v08_nca_quickpls.json").read_text(encoding="utf-8"))
    data = matrix(["x", "y"])
    reference = nca_reference(data[:, 0], data[:, 1])
    ceiling_by_name = {row["ceiling"]: row for row in nca["ceilings"]}
    numeric_differences = [
        abs(nca["scope"][field] - reference["scope"][field])
        for field in reference["scope"]
    ]
    numeric_differences.extend(
        abs(actual[field] - expected[index])
        for actual, expected in zip(nca["ce_fdh_peers"], reference["peers"])
        for index, field in enumerate(("x", "y"))
    )
    numeric_differences.extend([
        abs(ceiling_by_name["ce_fdh"]["effect_size"] - reference["ce_effect"]),
        abs(ceiling_by_name["cr_fdh"]["effect_size"] - reference["cr_effect"]),
        abs(ceiling_by_name["cr_fdh"]["slope"] - reference["slope"]),
        abs(ceiling_by_name["cr_fdh"]["intercept"] - reference["intercept"]),
    ])
    bottleneck_shape_matches = len(nca["bottlenecks"]) == len(reference["bottlenecks"])
    for actual, expected in zip(nca["bottlenecks"], reference["bottlenecks"]):
        bottleneck_shape_matches &= all(actual[field] == expected[field] for field in ("ceiling", "outcome_percent", "status"))
        if actual["required_x_percent"] is None or expected["required_x_percent"] is None:
            bottleneck_shape_matches &= actual["required_x_percent"] is expected["required_x_percent"]
        else:
            numeric_differences.append(abs(actual["required_x_percent"] - expected["required_x_percent"]))
    p_values = [row["permutation_p_value"] for row in nca["ceilings"]]
    p_value_lattice = all(0.01 <= value <= 1.0 and abs(value * 100.0 - round(value * 100.0)) <= 1e-9 for value in p_values)
    deterministic = nca == repeated
    exact_envelope = (
        envelope["provenance"]["method"] == "nca"
        and envelope["provenance"]["method_version"] == "nca_v2"
        and envelope["payload"]["kind"] == "pls_pm_v1"
        and envelope["payload"]["assessment"] == {
            "method_version": "assessment_not_applicable_v1",
            "warnings": ["PLS assessment is not applicable to standalone raw-data analyses."],
        }
        and "bootstrap" not in envelope["payload"]
        and "permutation" not in envelope["payload"]
    )
    exact_ceiling_shape = (
        [row["ceiling"] for row in nca["ceilings"]] == ["ce_fdh", "cr_fdh"]
        and ceiling_by_name["ce_fdh"]["slope"] is None
        and ceiling_by_name["ce_fdh"]["intercept"] is None
        and math.isfinite(ceiling_by_name["cr_fdh"]["slope"])
        and math.isfinite(ceiling_by_name["cr_fdh"]["intercept"])
    )
    ordered_peers = all(
        left["x"] < right["x"] and left["y"] < right["y"]
        for left, right in zip(nca["ce_fdh_peers"], nca["ce_fdh_peers"][1:])
    )
    max_difference = max(numeric_differences, default=0.0)
    return {
        "passed": (
            nca["method_version"] == "nca_v2"
            and len(nca["ce_fdh_peers"]) == len(reference["peers"])
            and max_difference <= TOL
            and bottleneck_shape_matches
            and p_value_lattice
            and deterministic
            and exact_envelope
            and exact_ceiling_shape
            and ordered_peers
        ),
        "method_version": nca["method_version"],
        "max_abs_difference": max_difference,
        "ce_fdh_peer_count": len(nca["ce_fdh_peers"]),
        "ce_fdh_peers_ordered": ordered_peers,
        "cr_fdh_regression_checked": True,
        "all_bottleneck_rows_checked": bottleneck_shape_matches,
        "permutation_p_value_lattice": p_value_lattice,
        "seeded_repeat_deterministic": deterministic,
        "exact_standalone_envelope": exact_envelope,
        "exact_ceiling_shape": exact_ceiling_shape,
        "scope": {
            "observed_numeric_variables": 2,
            "listwise_complete_rows_minimum": 3,
            "ceilings": ["ce_fdh", "cr_fdh"],
            "latent_score_nca": False,
            "multiple_conditions": False,
            "cipma": False,
        },
    }


def check_gsca():
    estimation = run_recipe("v08_gsca", "gsca", {}, model=gsca_model())
    gsca = estimation["gsca"]
    finite = all(math.isfinite(path["coefficient"]) for path in gsca["paths"])
    present = len(gsca["weights"]) == 5 and len(gsca["paths"]) == 1 and gsca["method_version"] == "gsca_v1"
    return {
        "passed": finite and present,
        "method_version": gsca["method_version"],
        "paths": len(gsca["paths"]),
        "weights": len(gsca["weights"]),
        "fit": gsca["fit"],
    }


CHECKS = {
    "pca": check_pca,
    "ols": check_ols,
    "logistic": check_logistic,
    "process": check_process,
    "nca": check_nca,
    "gsca": check_gsca,
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--section", choices=[*CHECKS.keys(), "all"], default="all")
    args = parser.parse_args()
    write_dataset()
    selected = CHECKS.keys() if args.section == "all" else [args.section]
    checks = {name: CHECKS[name]() for name in selected}
    nca_only = args.section == "nca"
    report = {
        "passed": all(item["passed"] for item in checks.values()),
        "schema_version": 1,
        "target": "nca_v2 bounded backend qualification" if nca_only else "v0.8 extended methods mixed-status reference",
        "selected_section": args.section,
        "tolerance": TOL,
        "checks": checks,
        "note": (
            "NCA v2 evidence is limited to the documented standalone observed numeric X/Y CE-FDH/CR-FDH scope; packaged-native acceptance and broader method parity are separate gates."
            if nca_only
            else "Each method keeps its own documented promotion status; this shared fixture does not promote broader variants."
        ),
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
