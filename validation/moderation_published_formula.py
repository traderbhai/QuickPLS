"""Published-formula fixture for two-stage moderation.

This is not a published-data replication. It freezes the standard moderated
regression equation y = b1*x + b2*m + b3*x*m + e on a fixed auditable table and
checks QuickPLS against independent standardized OLS equations.
"""

import json

import moderation_reference as ref


OUTPUT = ref.RESULTS / "moderation_published_formula_report.json"
TOLERANCE = 1e-10


def fixed_rows():
    pairs = [
        (-2.4, -1.7),
        (-2.1, -0.8),
        (-1.8, 0.2),
        (-1.4, 1.1),
        (-1.0, -1.3),
        (-0.7, -0.3),
        (-0.3, 0.8),
        (0.0, 1.6),
        (0.4, -1.5),
        (0.8, -0.6),
        (1.1, 0.4),
        (1.5, 1.3),
        (1.9, -1.0),
        (2.2, 0.1),
        (2.5, 1.2),
        (2.8, 1.8),
    ]
    deterministic_noise = [
        -0.03,
        0.02,
        0.01,
        -0.02,
        0.04,
        -0.01,
        0.02,
        -0.04,
        0.03,
        -0.02,
        0.01,
        0.02,
        -0.03,
        0.04,
        -0.01,
        0.00,
    ]
    rows = []
    for index, (x_value, m_value) in enumerate(pairs):
        y_value = (
            0.30 * x_value
            - 0.20 * m_value
            + 0.75 * x_value * m_value
            + deterministic_noise[index]
        )
        rows.append({"x": x_value, "m": m_value, "y": y_value})
    return rows


def main():
    rows = fixed_rows()
    result = ref.run_quickpls("moderation_published_formula", rows)
    reference = ref.independent_reference(rows)
    reference_delta = ref.max_path_difference(reference, result["paths"])
    slope_delta = ref.simple_slope_max_difference(reference, result)
    interaction = result["moderation"][0]
    checks = {
        "independent_reference_max_delta": reference_delta,
        "simple_slope_reference_max_delta": slope_delta,
        "interaction_effect_matches_path": abs(
            interaction["interaction_effect"] - result["paths"][("xm", "y")]
        ),
        "main_effect_x_matches_slope_at_mean_m": abs(
            interaction["simple_slopes"][1]["effect"] - result["paths"][("x", "y")]
        ),
        "moderator_low_slope": interaction["simple_slopes"][0]["effect"],
        "moderator_mean_slope": interaction["simple_slopes"][1]["effect"],
        "moderator_high_slope": interaction["simple_slopes"][2]["effect"],
        "experimental_warning_present": any(
            "Two-stage moderation is experimental" in warning for warning in result["warnings"]
        ),
    }
    passed = (
        checks["independent_reference_max_delta"] <= TOLERANCE
        and checks["simple_slope_reference_max_delta"] <= TOLERANCE
        and checks["interaction_effect_matches_path"] <= TOLERANCE
        and checks["main_effect_x_matches_slope_at_mean_m"] <= TOLERANCE
        and checks["experimental_warning_present"]
    )
    report = {
        "schema_version": 1,
        "kind": "pls_two_stage_moderation_published_formula_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "source_formula": "standard moderated regression y = b1*x + b2*m + b3*x*m + e",
        "source_formula_reference": {
            "baron_kenny_1986": "Baron & Kenny describe moderator effects through the interaction term in regression.",
            "aiken_west_1991": "Aiken & West provide the standard simple-slope interpretation for interactions.",
        },
        "rows": rows,
        "reference_paths": {
            f"{source}->{target}": value for (source, target), value in reference.items()
        },
        "quickpls_paths": {
            f"{source}->{target}": value for (source, target), value in result["paths"].items()
        },
        "checks": checks,
        "note": "Fixed-table published-formula fixture; it is not a published empirical dataset replication.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"reference_delta={reference_delta:.3g} | slope_delta={slope_delta:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
