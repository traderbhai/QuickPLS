"""Published empirical-data fixture for two-stage moderation.

The fixture uses the 32-row mtcars dataset distributed with R, originally
extracted from 1974 Motor Trend US magazine and documented by Henderson and
Velleman. Under QuickPLS' single-item two-stage moderation contract, the result
must match independent standardized OLS for mpg ~ wt + hp + wt*hp.
"""

import json

import moderation_reference as ref


OUTPUT = ref.RESULTS / "moderation_published_empirical_report.json"
TOLERANCE = 1e-10


def mtcars_rows():
    raw = [
        (21.0, 2.620, 110.0),
        (21.0, 2.875, 110.0),
        (22.8, 2.320, 93.0),
        (21.4, 3.215, 110.0),
        (18.7, 3.440, 175.0),
        (18.1, 3.460, 105.0),
        (14.3, 3.570, 245.0),
        (24.4, 3.190, 62.0),
        (22.8, 3.150, 95.0),
        (19.2, 3.440, 123.0),
        (17.8, 3.440, 123.0),
        (16.4, 4.070, 180.0),
        (17.3, 3.730, 180.0),
        (15.2, 3.780, 180.0),
        (10.4, 5.250, 205.0),
        (10.4, 5.424, 215.0),
        (14.7, 5.345, 230.0),
        (32.4, 2.200, 66.0),
        (30.4, 1.615, 52.0),
        (33.9, 1.835, 65.0),
        (21.5, 2.465, 97.0),
        (15.5, 3.520, 150.0),
        (15.2, 3.435, 150.0),
        (13.3, 3.840, 245.0),
        (19.2, 3.845, 175.0),
        (27.3, 1.935, 66.0),
        (26.0, 2.140, 91.0),
        (30.4, 1.513, 113.0),
        (15.8, 3.170, 264.0),
        (19.7, 2.770, 175.0),
        (15.0, 3.570, 335.0),
        (21.4, 2.780, 109.0),
    ]
    return [{"y": mpg, "x": wt, "m": hp} for mpg, wt, hp in raw]


def main():
    rows = mtcars_rows()
    result = ref.run_quickpls("moderation_published_empirical_mtcars", rows)
    reference = ref.independent_reference(rows)
    reference_delta = ref.max_path_difference(reference, result["paths"])
    slope_delta = ref.simple_slope_max_difference(reference, result)
    interaction = result["moderation"][0]
    checks = {
        "row_count": len(rows),
        "used_observations": result["used_observations"],
        "omitted_observations": result["omitted_observations"],
        "independent_reference_max_delta": reference_delta,
        "simple_slope_reference_max_delta": slope_delta,
        "interaction_effect_matches_path": abs(
            interaction["interaction_effect"] - result["paths"][("xm", "y")]
        ),
        "main_effect_wt_matches_slope_at_mean_hp": abs(
            interaction["simple_slopes"][1]["effect"] - result["paths"][("x", "y")]
        ),
        "moderator_low_hp_slope": interaction["simple_slopes"][0]["effect"],
        "moderator_mean_hp_slope": interaction["simple_slopes"][1]["effect"],
        "moderator_high_hp_slope": interaction["simple_slopes"][2]["effect"],
        "interaction_abs": abs(result["paths"][("xm", "y")]),
        "experimental_warning_present": any(
            "Two-stage moderation is experimental" in warning for warning in result["warnings"]
        ),
    }
    passed = (
        checks["row_count"] == 32
        and checks["used_observations"] == 32
        and checks["omitted_observations"] == 0
        and checks["independent_reference_max_delta"] <= TOLERANCE
        and checks["simple_slope_reference_max_delta"] <= TOLERANCE
        and checks["interaction_effect_matches_path"] <= TOLERANCE
        and checks["main_effect_wt_matches_slope_at_mean_hp"] <= TOLERANCE
        and checks["interaction_abs"] >= 0.05
        and checks["experimental_warning_present"]
    )
    report = {
        "schema_version": 1,
        "kind": "pls_two_stage_moderation_published_empirical_mtcars_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "dataset": {
            "name": "mtcars",
            "rows": 32,
            "source": "R datasets package; extracted from 1974 Motor Trend US magazine.",
            "documentation_reference": "Henderson and Velleman (1981), Building multiple regression models interactively.",
            "columns": {
                "y": "mpg",
                "x": "wt",
                "m": "hp",
            },
        },
        "model": "single-item two-stage moderation: mpg ~ wt + hp + wt_by_hp",
        "reference_paths": {
            f"{source}->{target}": value for (source, target), value in reference.items()
        },
        "quickpls_paths": {
            f"{source}->{target}": value for (source, target), value in result["paths"].items()
        },
        "checks": checks,
        "note": "Published empirical-data contract fixture. It verifies QuickPLS' frozen single-item two-stage moderation implementation against independent standardized OLS; it is not a full inferential coverage study.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"reference_delta={reference_delta:.3g} | slope_delta={slope_delta:.3g} | "
        f"interaction={result['paths'][('xm', 'y')]:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
