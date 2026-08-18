from __future__ import annotations

import copy
import hashlib
import json
import math
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

import pytest
import numpy as np
from jsonschema import Draft202012Validator, ValidationError

from validation.cbsem_exact_cfa_interval_coverage_v1 import (
    CASE_SCHEMA,
    DEFAULT_MANIFEST,
    MANIFEST_SCHEMA,
    REPORT_SCHEMA,
    ROOT,
    aggregate_case_documents,
    bonferroni_clopper_pearson_lower,
    bonferroni_clopper_pearson_two_sided,
    clopper_pearson_two_sided,
    compact_evidence_payload,
    derive_seed,
    dataset_cluster_failure_summary,
    complete_case_universe_sha256,
    generate_dataset_csv,
    plan_shards,
    sampling_positions_sha256,
    source_rows_sha256,
    typed_json_sha256,
    _binomial_endpoint_is_conservative,
    _certified_clopper_pearson_bounds,
    _dataset_seeds,
    _expected_dataset_uuid,
    validate_engine_document,
    validate_manifest,
)
from validation import cbsem_exact_cfa_interval_coverage_v1_oracle as oracle_module
from validation import cbsem_exact_cfa_interval_coverage_v1 as coverage_module
from validation.cbsem_exact_cfa_interval_coverage_v1_oracle import (
    audit_engine_case,
    bonferroni_clopper_pearson_lower as oracle_lower,
    bonferroni_clopper_pearson_two_sided as oracle_simultaneous,
    clopper_pearson_two_sided as oracle_two_sided,
    dataset_cluster_failure_summary as oracle_cluster_summary,
)


def test_manifest_binds_frozen_57_cell_program_and_blocks_qualification() -> None:
    manifest, digest = validate_manifest(DEFAULT_MANIFEST)
    assert len(digest) == 64
    assert [len(dgp["parameter_truth"]) for dgp in manifest["dgps"]] == [6, 13]
    assert (6 + 13) * 3 == manifest["coverage_contract"]["primary_cell_count"] == 57
    assert manifest["coverage_contract"]["availability_cell_count"] == 6
    assert manifest["digest_contract"]["compact_evidence_fields"] == [
        "scientific_result_sha256",
        "data_sha256",
        "source_dataset_fingerprint",
        "ledger",
        "methods",
    ]
    assert manifest["digest_contract"]["coverage_authentication"] == (
        "excluded_from_compact_evidence_and_rederived_by_aggregator_v1"
    )
    assert manifest["binary_binding"]["sha256"] == "e222043929361e5b0fe14990e89e83b7f6bf98c11c9413c42e6bb4b16e6f2fb3"
    validate_manifest(DEFAULT_MANIFEST, require_binary=True)
    assert manifest["scientific_runtime"] == {
        "python": "3.13.1",
        "numpy": "1.26.4",
        "scipy": "1.15.2",
    }
    assert manifest["coverage_contract"]["binomial_endpoint_kernel"] == (
        "scipy_beta_seed_exact_integer_binomial_tail_certified_tight_binary64_v2"
    )
    assert manifest["coverage_contract"]["ordinary_two_sided_tail"] == {
        "numerator": 1,
        "denominator": 40,
    }
    assert manifest["coverage_contract"]["simultaneous_two_sided_tail"] == {
        "numerator": 1,
        "denominator": 2280,
    }
    assert manifest["coverage_contract"]["availability_one_sided_tail"] == {
        "numerator": 1,
        "denominator": 120,
    }
    assert coverage_module.scientific_runtime_versions() == manifest["scientific_runtime"]
    validate_manifest(DEFAULT_MANIFEST, require_scientific_runtime=True)
    assert manifest["qualification_boundary"]["status"] == "blocked_not_product_qualification"


@pytest.mark.parametrize("field", ["python", "numpy", "scipy"])
def test_execution_rejects_local_scientific_runtime_drift(
    field: str, monkeypatch: pytest.MonkeyPatch
) -> None:
    observed = coverage_module.scientific_runtime_versions()
    drifted = {**observed, field: f"{observed[field]}-drifted"}
    monkeypatch.setattr(
        coverage_module, "scientific_runtime_versions", lambda: drifted
    )
    with pytest.raises(ValueError, match="scientific runtime differs"):
        validate_manifest(DEFAULT_MANIFEST, require_scientific_runtime=True)


def test_seed_domains_and_pcg64dxsm_fixture_are_frozen() -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    observed = {
        purpose: derive_seed(
            manifest["seed_contract"][f"{purpose}_master_seed"],
            "dgp_a_one_factor_n150",
            "conformance",
            0,
            purpose,
        )
        for purpose in ("data", "bootstrap", "oracle")
    }
    assert observed == {
        "data": 3324661327777498169,
        "bootstrap": 13168175097069679967,
        "oracle": 14295253035839127755,
    }
    assert len(set(observed.values())) == 3
    payload = generate_dataset_csv(manifest["dgps"][0], observed["data"])
    assert hashlib.sha256(payload).hexdigest() == "c89aa6501c26519d4604cdde499c001b38686af752358be2ae69260707532ce4"
    assert len(payload.splitlines()) == 151


def test_executed_rust_model_never_receives_population_truth_starts() -> None:
    source = (ROOT / "crates/qpls-resampling/examples/cbsem_exact_cfa_interval_coverage_v1.rs").read_text(encoding="utf-8")
    assert "apply_true_starts" not in source
    assert "start: dgp.factor_covariance" not in source
    assert "*start = Some" not in source
    assert "population truth is kept in a separate reporting map" in source


def test_shard_planner_freezes_only_conformance_and_pilot_as_executable() -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    conformance = plan_shards(manifest, "conformance")
    pilot = plan_shards(manifest, "pilot")
    final = plan_shards(manifest, "final")
    confirmation = plan_shards(manifest, "paired_confirmation")
    assert (len(conformance), len(pilot), len(final), len(confirmation)) == (2, 8, 64, 2)
    assert all(row["executable"] for row in conformance + pilot)
    assert not any(row["executable"] for row in final + confirmation)
    assert all(len(row["dataset_indices"]) == 100 for row in final)
    assert {index for row in final if row["dgp_id"] == "dgp_a_one_factor_n150" for index in row["dataset_indices"]} == set(range(3200))
    for dgp in manifest["dgps"]:
        for index in range(20):
            assert _dataset_seeds(manifest, dgp["dgp_id"], "paired_confirmation", index) == _dataset_seeds(
                manifest, dgp["dgp_id"], "final", index
            )


@pytest.mark.parametrize(
    "successes,trials",
    [(0, 20), (1, 20), (19, 20), (20, 20), (3040, 3200), (3200, 3200)],
)
def test_certified_exact_binomial_bounds_match_independent_scipy_oracle(
    successes: int, trials: int
) -> None:
    observed = clopper_pearson_two_sided(successes, trials)
    expected = oracle_two_sided(successes, trials)
    simultaneous = bonferroni_clopper_pearson_two_sided(successes, trials, 57)
    expected_simultaneous = oracle_simultaneous(successes, trials, 57)
    availability = bonferroni_clopper_pearson_lower(successes, trials, 6)
    expected_availability = oracle_lower(successes, trials, 6)
    assert _bounds_close(observed, expected)
    assert _bounds_close(simultaneous, expected_simultaneous)
    assert availability == pytest.approx(expected_availability, rel=1e-10, abs=1e-12)


def _brute_fraction_binomial_tail(
    successes: int, trials: int, probability: float, endpoint: str
) -> Fraction:
    p = Fraction(*probability.as_integer_ratio())
    q = 1 - p
    if endpoint == "lower":
        indices = range(successes, trials + 1)
    elif endpoint == "upper":
        indices = range(successes + 1)
    else:
        raise ValueError(endpoint)
    return sum(
        Fraction(math.comb(trials, index)) * p**index * q ** (trials - index)
        for index in indices
    )


def _assert_tight_certified_bounds(
    successes: int, trials: int, tail: tuple[int, int]
) -> None:
    lower, upper = _certified_clopper_pearson_bounds(successes, trials, *tail)
    tail_fraction = Fraction(*tail)
    if successes == 0:
        assert lower == 0.0
    else:
        assert _binomial_endpoint_is_conservative(
            successes, trials, lower, *tail, "lower"
        )
        assert not _binomial_endpoint_is_conservative(
            successes, trials, math.nextafter(lower, 1.0), *tail, "lower"
        )
        if trials <= 20:
            assert _brute_fraction_binomial_tail(
                successes, trials, lower, "lower"
            ) <= tail_fraction
            assert _brute_fraction_binomial_tail(
                successes, trials, math.nextafter(lower, 1.0), "lower"
            ) > tail_fraction
    if successes == trials:
        assert upper == 1.0
    else:
        assert _binomial_endpoint_is_conservative(
            successes, trials, upper, *tail, "upper"
        )
        assert not _binomial_endpoint_is_conservative(
            successes, trials, math.nextafter(upper, 0.0), *tail, "upper"
        )
        if trials <= 20:
            assert _brute_fraction_binomial_tail(
                successes, trials, upper, "upper"
            ) <= tail_fraction
            assert _brute_fraction_binomial_tail(
                successes, trials, math.nextafter(upper, 0.0), "upper"
            ) > tail_fraction


@pytest.mark.parametrize("successes", range(21))
@pytest.mark.parametrize("tail", [(1, 40), (1, 2280), (1, 120)])
def test_all_pilot_counts_have_exact_outward_tight_binary64_bounds(
    successes: int, tail: tuple[int, int]
) -> None:
    _assert_tight_certified_bounds(successes, 20, tail)


@pytest.mark.parametrize("successes", [3008, 3009, 3088, 3089])
@pytest.mark.parametrize("tail", [(1, 40), (1, 2280)])
def test_final_threshold_neighbors_have_exact_outward_tight_binary64_bounds(
    successes: int, tail: tuple[int, int]
) -> None:
    _assert_tight_certified_bounds(successes, 3200, tail)


def test_exact_binomial_boundaries_invalid_inputs_and_cache() -> None:
    assert clopper_pearson_two_sided(0, 20)[0] == 0.0
    assert clopper_pearson_two_sided(20, 20)[1] == 1.0
    for args in [(-1, 20), (21, 20), (0, 0)]:
        with pytest.raises(ValueError):
            clopper_pearson_two_sided(*args)
    for confidence in [0.0, 1.0, math.nan, True]:
        with pytest.raises(ValueError):
            clopper_pearson_two_sided(19, 20, confidence)
    with pytest.raises(ValueError):
        bonferroni_clopper_pearson_two_sided(19, 20, 0)
    with pytest.raises(ValueError):
        bonferroni_clopper_pearson_lower(19, 20, 0)
    for tail in [(0, 40), (1, 1), (2, 40)]:
        with pytest.raises(ValueError):
            _certified_clopper_pearson_bounds(19, 20, *tail)
    for successes, trials in [(-1, 20), (21, 20), (0, 0)]:
        with pytest.raises(ValueError):
            _certified_clopper_pearson_bounds(successes, trials, 1, 40)
    with pytest.raises(ValueError):
        _binomial_endpoint_is_conservative(19, 20, math.nan, 1, 40, "lower")
    with pytest.raises(ValueError):
        _binomial_endpoint_is_conservative(19, 20, 0.9, 1, 40, "unknown")
    _certified_clopper_pearson_bounds.cache_clear()
    first = _certified_clopper_pearson_bounds(19, 20, 1, 2280)
    second = _certified_clopper_pearson_bounds(19, 20, 1, 2280)
    assert first == second
    assert _certified_clopper_pearson_bounds.cache_info().hits == 1


def test_dataset_cluster_failure_summary_matches_independent_scipy_oracle() -> None:
    values = [0.0, 0.001, 0.004, 0.002, 0.0, 0.007, 0.003, 0.001] * 3
    observed = dataset_cluster_failure_summary(values, 991827, 10_000, 0.95)
    expected = oracle_cluster_summary(values, 991827, 10_000, 0.95)
    assert observed == pytest.approx(expected, rel=1e-12, abs=1e-12)
    assert (observed["p05"], observed["p50"], observed["p95"]) == pytest.approx(
        [0.0, 0.0015, 0.007], rel=0.0, abs=1e-15
    )


def test_conformance_oracle_accepts_complete_ledgers_and_no_endpoints() -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    document, _, _, _, _ = _conformance_document(manifest)
    receipt = audit_engine_case(document)
    assert receipt["status"] == "accepted"
    assert receipt["endpoint_checks"] == 0
    assert not receipt["base_inference_available"]
    assert not receipt["studentized_inference_available"]
    assert not receipt["bca_inference_available"]


def test_oracle_rejects_coherent_intermediate_tamper_not_just_endpoint_tamper() -> None:
    document = _available_oracle_document()
    receipt = audit_engine_case(document)
    assert receipt["status"] == "accepted"
    assert receipt["endpoint_checks"] == 6
    assert receipt["arithmetic_checks"] == 13

    tampered_documents = []
    percentile = copy.deepcopy(document)
    percentile["bootstrap"]["base"]["intervals"][0]["bootstrap_mean"] += 0.01
    tampered_documents.append(percentile)
    studentized = copy.deepcopy(document)
    studentized["bootstrap"]["studentized"]["intervals"][0]["outcome"][
        "lower_pivot_quantile"
    ] += 0.01
    tampered_documents.append(studentized)
    bca = copy.deepcopy(document)
    bca["bca"]["intervals"][0]["outcome"]["acceleration"] += 0.01
    tampered_documents.append(bca)
    for tampered in tampered_documents:
        rejected = audit_engine_case(tampered)
        assert rejected["status"] == "rejected"
        assert any("arithmetic_mismatch" in reason for reason in rejected["reasons"])


def test_percentile_naive_mean_bound_is_outward_rigorous_and_bias_is_bit_bound() -> None:
    # This exact binary64 vector has severe ordered cancellation: the naïve
    # engine mean is 0.75 while the exact mathematical mean is 1.0.
    values = [1.0e16, 1.0, -1.0e16, 3.0]
    running = 0.0
    for value in values:
        running += value
    observed_mean = running / len(values)
    point = 0.5
    observed_bias = observed_mean - point
    reference_mean, reference_bias, mean_bound, bias_bound = (
        oracle_module._percentile_mean_bias_reference(
            values, observed_mean, observed_bias, point
        )
    )

    exact_mean = sum((Fraction.from_float(value) for value in values), Fraction()) / len(
        values
    )
    exact_bias = exact_mean - Fraction.from_float(point)
    assert abs(Fraction.from_float(observed_mean) - exact_mean) <= Fraction.from_float(
        mean_bound
    )
    assert abs(Fraction.from_float(observed_bias) - exact_bias) <= Fraction.from_float(
        bias_bound
    )
    assert abs(Fraction.from_float(reference_mean) - exact_mean) <= Fraction.from_float(
        math.ulp(reference_mean)
    )
    assert abs(Fraction.from_float(reference_bias) - exact_bias) <= Fraction.from_float(
        math.ulp(reference_bias)
    )

    # The first representable value strictly beyond the outward envelope is
    # rejected even when its bias is coherently recomputed.
    outside_mean = math.nextafter(reference_mean + mean_bound, math.inf)
    reasons: list[str] = []
    oracle_module._compare_percentile_mean_bias(
        outside_mean,
        outside_mean - point,
        values,
        point,
        "percentile:bound",
        reasons,
    )
    assert "arithmetic_mismatch:percentile:bound:bootstrap_mean" in reasons

    # A one-ULP serialized bias splice cannot hide inside the theorem bound.
    reasons = []
    oracle_module._compare_percentile_mean_bias(
        observed_mean,
        math.nextafter(observed_bias, math.inf),
        values,
        point,
        "percentile:bias",
        reasons,
    )
    assert "arithmetic_mismatch:percentile:bias:bias_binary64_binding" in reasons

    # Exercise a subtraction across the 2.0 binade where the exact reference
    # bias lies between binary64 values. Both bias-center ULPs are included
    # outwardly; the serialized bias remains bound to the observed mean.
    binade_values = [2.0, 2.0, 2.0, 2.0]
    binade_point = 2.0**-54
    binade_mean = math.nextafter(2.0, math.inf)
    binade_bias = binade_mean - binade_point
    binade_reference_mean, binade_reference_bias, binade_mean_bound, binade_bias_bound = (
        oracle_module._percentile_mean_bias_reference(
            binade_values, binade_mean, binade_bias, binade_point
        )
    )
    assert binade_reference_mean == 2.0
    assert binade_reference_bias == 2.0
    assert binade_bias_bound > (
        binade_mean_bound + math.ulp(binade_bias) + math.ulp(binade_reference_bias)
    )
    assert abs(binade_bias - binade_reference_bias) <= binade_bias_bound


def test_percentile_mean_bound_fails_closed_for_nonfinite_or_overflowing_ledgers() -> None:
    for values in ([1.0, math.inf], [sys.float_info.max, sys.float_info.max]):
        with pytest.raises(ValueError):
            oracle_module._percentile_mean_bias_reference(values, 1.0, 0.0, 1.0)


def test_rejected_pilot_ledger_is_append_only_and_now_isolates_historical_bca_cdf() -> None:
    rejected = (
        ROOT
        / "validation/results/cbsem_exact_cfa_interval_coverage_v1"
        / "coverage-pilot-20260816-a/shards/pilot-dgp_a_one_factor_n150-s00"
    )
    expected_hashes = {
        "audit/dataset-000000.json": "4d0fccd983016df511390b20ec3910536c016db5e53679c2957dfbea565f4e80",
        "audit/dataset-000001.json": "f557e4cc237cb0cef14d6f6018c28f569019373f508c1b42061042280b907ffb",
        "audit/dataset-000002.json": "3073fb04d1827c84c30463e2b160b29d5bab9d99d770307fb671154bc9c5fd86",
        "audit/dataset-000003.json": "fa10d00ddfd0c9142ffd073690d07e89bd22a1cd10b8633762a4da035297d2ff",
        "audit/dataset-000004.json": "9f7f51a2ca5a181e69f253e2333dc1a19ec577c1941a3e96fb4f11346fcd7a7a",
        "case.json": "f2ebe4de8d51931e5b9cfd69486d48c5016249d573b488945d540d0a6bc27f59",
    }
    observed_files = {
        path.relative_to(rejected).as_posix(): path
        for path in rejected.rglob("*")
        if path.is_file()
    }
    assert set(observed_files) == set(expected_hashes)
    for relative, expected_hash in expected_hashes.items():
        assert hashlib.sha256(observed_files[relative].read_bytes()).hexdigest() == expected_hash

    for relative in sorted(name for name in expected_hashes if name.startswith("audit/")):
        document = json.loads(observed_files[relative].read_text(encoding="utf-8"))
        assert document["bca"]["method_version"] == "cbsem_exact_case_bootstrap_bca_interval_v1"
        assert document["bca"]["acceleration_method"] == (
            "complete_delete_one_jackknife_acceleration_v1"
        )
        assert document["bca"]["adjusted_probability_method"] == (
            "efron_bca_normal_cdf_adjustment_v1"
        )
        receipt = audit_engine_case(document)
        assert receipt["status"] == "rejected"
        assert receipt["reasons"]
        assert all(
            reason.startswith("arithmetic_mismatch:") and ":bca:" in reason
            for reason in receipt["reasons"]
        )
        assert not any("percentile" in reason for reason in receipt["reasons"])


def test_pilot_c_evidence_is_append_only_under_the_superseded_cp_kernel() -> None:
    preserved = (
        ROOT
        / "validation/results/cbsem_exact_cfa_interval_coverage_v1"
        / "coverage-pilot-20260816-c"
    )
    expected_hashes = {
        "pilot-report.json": "94ab5a0ac4448f39f809ead3fcb746287bd16da1a15e6c7e65ffa07d7d32464c",
        "shards/pilot-dgp_a_one_factor_n150-s00/audit/dataset-000000.json": "f8f166edcfc7022cbbbd8059fe3b8561d82073346bfa913eec1b65726dcc0ade",
        "shards/pilot-dgp_a_one_factor_n150-s00/case.json": "7ace93a9178621d9a479323d8c66b44125257bfa2977dbdffc3e902b52110807",
        "shards/pilot-dgp_a_one_factor_n150-s01/audit/dataset-000005.json": "97597a7324ec260787288a439008d2ac0431fd9abe0a6048efb4487d8cf7761c",
        "shards/pilot-dgp_a_one_factor_n150-s01/case.json": "c61c554ac798e2465c131afaab4b7192e2c215f2f33a9f9a2e7576f289825004",
        "shards/pilot-dgp_a_one_factor_n150-s02/audit/dataset-000010.json": "976267d7438ead8b4350b273144406924bc5e3b3e07e3b54940d73064c1a8752",
        "shards/pilot-dgp_a_one_factor_n150-s02/case.json": "ef5fb4b477acbf48b88be3b6161c89c9295469d70eaa2c2cdf1994c99fb57c94",
        "shards/pilot-dgp_a_one_factor_n150-s03/audit/dataset-000015.json": "82321b3af25e8d78c51ce8de0d4abf0d51a08a0ff831988425f53b6667e31e30",
        "shards/pilot-dgp_a_one_factor_n150-s03/case.json": "093335107f2420c48cddd0e41860b59738366aee915b69857ac5e3bf0d94c8e2",
        "shards/pilot-dgp_b_two_factor_n300-s00/audit/dataset-000000.json": "4e3de32d893595b83d6d5f1e25b547fa2da06aa48f56b0588743073c42647a6c",
        "shards/pilot-dgp_b_two_factor_n300-s00/case.json": "df680476ff4e249b09a6dbae05e8c1c12baa2b04037c0f13ce1200ffce3f98a8",
        "shards/pilot-dgp_b_two_factor_n300-s01/audit/dataset-000005.json": "58098f83811d08d8cbb4030c9b86fdb527daeffdf46d4cbff32c4c182281a56a",
        "shards/pilot-dgp_b_two_factor_n300-s01/case.json": "bc5ac457cadb1d61bcfd6f457e37cf0d472709549e0b81b14fc8d7aa8cde1088",
        "shards/pilot-dgp_b_two_factor_n300-s02/audit/dataset-000010.json": "ca0a9c37510a010fc1781e8aaecd4374e0e5d0d237bd34d88bc2c79617fcaadf",
        "shards/pilot-dgp_b_two_factor_n300-s02/case.json": "4fed50bdb638af6a6e927cfde32091a1bb14253be1c46fc97ff485ba9637c003",
        "shards/pilot-dgp_b_two_factor_n300-s03/audit/dataset-000015.json": "61fbd3f488a2a3c3326991073300275ac605961960887d5c606dc13a577f8340",
        "shards/pilot-dgp_b_two_factor_n300-s03/case.json": "fdd14cf09dafaf498a3f0d585307b80d9235b1c562b29a82983e3757388a2505",
    }
    observed = {
        path.relative_to(preserved).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in preserved.rglob("*")
        if path.is_file()
    }
    assert observed == expected_hashes
    historical_report = json.loads(
        (preserved / "pilot-report.json").read_text(encoding="utf-8")
    )
    for row in historical_report["primary_cells"]:
        corrected = _certified_clopper_pearson_bounds(
            row["covered_unconditional"], row["requested_datasets"], 1, 2280
        )
        assert corrected[0] < row["simultaneous_bonferroni_cp_lower"]


def test_resampling_lock_resolves_the_direct_cdf_kernel_to_exact_libm_0_2_16() -> None:
    cargo = (ROOT / "crates/qpls-resampling/Cargo.toml").read_text(encoding="utf-8")
    lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
    assert 'libm = "=0.2.16"' in cargo
    assert 'name = "libm"\nversion = "0.2.16"' in lock
    qpls_resampling = lock.split('name = "qpls-resampling"', 1)[1].split(
        "[[package]]", 1
    )[0]
    assert '\n "libm",\n' in qpls_resampling


def test_engine_validator_accepts_full_conformance_partition_and_rejects_tamper() -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    document, dgp, bootstrap_seed, data_sha, authentication = _conformance_document(manifest)
    receipt = validate_engine_document(
        document, manifest, "conformance", dgp, 0, bootstrap_seed, data_sha, authentication
    )
    assert receipt["status"] == "accepted"
    assert receipt["ledger"]["point_successes"] == 500
    assert receipt["ledger"]["delete_one_successes"] == 150
    assert all(not row["available"] for row in receipt["methods"])
    assert all(
        parameter["covered_unconditional"] is None
        for method in receipt["methods"]
        for parameter in method["parameters"]
    )

    duplicate = copy.deepcopy(document)
    duplicate["bootstrap"]["base"]["successful_refits"][1]["replicate_index"] = 0
    duplicate_auth = _refresh_scientific_authentication(
        duplicate, data_sha, "1" * 64, 500, 150
    )
    with pytest.raises(ValueError, match="exact one-attempt partition"):
        validate_engine_document(
            duplicate, manifest, "conformance", dgp, 0, bootstrap_seed, data_sha, duplicate_auth
        )

    reordered = copy.deepcopy(document)
    reordered["bootstrap"]["base"]["successful_refits"][:2] = reversed(
        reordered["bootstrap"]["base"]["successful_refits"][:2]
    )
    reordered_auth = _refresh_scientific_authentication(
        reordered, data_sha, "1" * 64, 500, 150
    )
    with pytest.raises(ValueError, match="exact one-attempt partition"):
        validate_engine_document(
            reordered,
            manifest,
            "conformance",
            dgp,
            0,
            bootstrap_seed,
            data_sha,
            reordered_auth,
        )

    wrong_b = copy.deepcopy(document)
    wrong_b["bootstrap"]["base"]["requested_replicates"] = 499
    wrong_b_auth = _refresh_scientific_authentication(
        wrong_b, data_sha, "1" * 64, 500, 150
    )
    with pytest.raises(ValueError, match="scientific identity|exact one-attempt partition"):
        validate_engine_document(
            wrong_b,
            manifest,
            "conformance",
            dgp,
            0,
            bootstrap_seed,
            data_sha,
            wrong_b_auth,
        )

    untyped = copy.deepcopy(document)
    failed = untyped["bootstrap"]["base"]["successful_refits"].pop()
    untyped["bootstrap"]["base"]["usable_replicates"] -= 1
    untyped["bootstrap"]["base"]["failed_replicates"] = 1
    untyped["bootstrap"]["base"]["failed_refits"] = [{
        "replicate_index": failed["replicate_index"],
        "sampling_positions_sha256": "0" * 64,
        "sample_indices_sha256": "0" * 64,
        "kind": "mystery",
        "message": "typed evidence required",
    }]
    untyped_auth = _refresh_scientific_authentication(
        untyped, data_sha, "1" * 64, 500, 150
    )
    with pytest.raises(ValueError, match="untyped"):
        validate_engine_document(
            untyped, manifest, "conformance", dgp, 0, bootstrap_seed, data_sha, untyped_auth
        )

    scientific_tamper = copy.deepcopy(document)
    scientific_tamper["bootstrap"]["base"]["successful_refits"][0]["objective"] = 0.25
    with pytest.raises(ValueError, match="scientific result digest"):
        validate_engine_document(
            scientific_tamper,
            manifest,
            "conformance",
            dgp,
            0,
            bootstrap_seed,
            data_sha,
            authentication,
        )

    base_point_tamper = copy.deepcopy(document)
    base_point_tamper["bootstrap"]["base"]["base_point_result_sha256"] = "f" * 64
    base_point_tamper["bca"]["base_point_result_sha256"] = "f" * 64
    base_point_auth = _refresh_scientific_authentication(
        base_point_tamper, data_sha, "1" * 64, 500, 150
    )
    with pytest.raises(ValueError, match="base-point digest"):
        validate_engine_document(
            base_point_tamper,
            manifest,
            "conformance",
            dgp,
            0,
            bootstrap_seed,
            data_sha,
            base_point_auth,
        )

    source_digest_tamper = copy.deepcopy(document)
    source_digest_tamper["original"]["refit"]["sample_indices_sha256"] = "0" * 64
    source_digest_tamper["bootstrap"]["base"]["base_point_result_sha256"] = typed_json_sha256(
        source_digest_tamper["original"]["refit"]
    )
    source_digest_tamper["bca"]["base_point_result_sha256"] = source_digest_tamper[
        "bootstrap"
    ]["base"]["base_point_result_sha256"]
    source_digest_auth = _refresh_scientific_authentication(
        source_digest_tamper, data_sha, "1" * 64, 500, 150
    )
    with pytest.raises(ValueError, match="source-row digest"):
        validate_engine_document(
            source_digest_tamper,
            manifest,
            "conformance",
            dgp,
            0,
            bootstrap_seed,
            data_sha,
            source_digest_auth,
        )

    schedule_tamper = copy.deepcopy(document)
    schedule_tamper["bootstrap"]["base"]["successful_refits"][0][
        "sampling_positions_sha256"
    ] = "0" * 64
    schedule_tamper["scientific_result_sha256"] = typed_json_sha256(
        {
            "original": schedule_tamper["original"],
            "bootstrap": schedule_tamper["bootstrap"],
            "bca": schedule_tamper["bca"],
        }
    )
    with pytest.raises(ValueError, match="verification receipt"):
        validate_engine_document(
            schedule_tamper,
            manifest,
            "conformance",
            dgp,
            0,
            bootstrap_seed,
            data_sha,
            authentication,
        )


def test_rust_verifier_and_python_share_exact_compact_evidence_digest(
    tmp_path: Path,
) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    document, dgp, _, data_sha, _ = _conformance_document(manifest)
    seeds = _dataset_seeds(manifest, dgp["dgp_id"], "conformance", 0)
    data_path = tmp_path / "fixture.csv"
    document_path = tmp_path / "document.json"
    data_path.write_bytes(generate_dataset_csv(dgp, seeds["data"]))
    document_path.write_text(json.dumps(document), encoding="utf-8")
    process = subprocess.run(
        [
            str(ROOT / manifest["binary_binding"]["path"]),
            "--verify-document",
            str(document_path),
            "--data",
            str(data_path),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert process.returncode == 0, process.stderr
    receipt = json.loads(process.stdout)
    assert receipt["scientific_result_sha256"] == document["scientific_result_sha256"]
    assert receipt["data_sha256"] == data_sha
    expected = _digest_authentication(
        document,
        data_sha,
        receipt["source_dataset_fingerprint"],
        500,
        dgp["sample_size"],
    )
    assert receipt["compact_evidence_typed_sha256"] == expected[
        "compact_evidence_typed_sha256"
    ]


def test_pilot_aggregate_has_57_cells_and_unavailable_is_not_covered(tmp_path: Path) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    manifest["binary_binding"]["sha256"] = "a" * 64
    manifest_sha = "b" * 64
    cases = [_synthetic_pilot_case(manifest, manifest_sha, shard, tmp_path) for shard in plan_shards(manifest, "pilot")]
    report = aggregate_case_documents(
        manifest, manifest_sha, "pilot", cases, evidence_root=tmp_path
    )
    Draft202012Validator(json.loads(REPORT_SCHEMA.read_text(encoding="utf-8"))).validate(report)
    assert report["status"] == "pilot_evidence_assembled_no_qualification_decision_product_qualification_blocked"
    assert len(report["primary_cells"]) == 57
    assert len(report["availability_cells"]) == 6
    target = next(
        row
        for row in report["primary_cells"]
        if row["dgp_id"] == "dgp_a_one_factor_n150"
        and row["method"] == "bca"
        and row["parameter_id"] == manifest["dgps"][0]["parameter_truth"][0]["parameter_id"]
    )
    assert target["requested_datasets"] == 20
    assert target["available_datasets"] == 19
    assert target["covered_unconditional"] == 19
    assert target["covered_conditional"] == 19
    assert target["unconditional_rate"] == 0.95
    assert target["conditional_rate"] == 1.0
    assert target["final_acceptance"] is None
    assert report["coverage_decision"] == "descriptive_pilot_only_no_qualification_decision"


def test_aggregate_rejects_missing_or_duplicate_shards(tmp_path: Path) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    manifest["binary_binding"]["sha256"] = "a" * 64
    manifest_sha = "b" * 64
    cases = [_synthetic_pilot_case(manifest, manifest_sha, shard, tmp_path) for shard in plan_shards(manifest, "pilot")]
    with pytest.raises(ValueError, match="duplicated or incomplete"):
        aggregate_case_documents(manifest, manifest_sha, "pilot", cases[:-1], evidence_root=tmp_path)
    with pytest.raises(ValueError, match="duplicated or incomplete"):
        aggregate_case_documents(manifest, manifest_sha, "pilot", cases + [cases[0]], evidence_root=tmp_path)


def test_aggregate_rejects_planned_shard_seed_replay_and_retained_hash_tamper(
    tmp_path: Path,
) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    manifest["binary_binding"]["sha256"] = "a" * 64
    manifest_sha = "b" * 64
    cases = [
        _synthetic_pilot_case(manifest, manifest_sha, shard, tmp_path)
        for shard in plan_shards(manifest, "pilot")
    ]

    shard_tamper = copy.deepcopy(cases)
    shard_tamper[1]["shard"]["shard_ordinal"] += 1
    with pytest.raises(ValueError, match="immutable plan"):
        aggregate_case_documents(
            manifest, manifest_sha, "pilot", shard_tamper, evidence_root=tmp_path
        )

    seed_tamper = copy.deepcopy(cases)
    seed_tamper[1]["dataset_receipts"][0]["seeds"]["data"] += 1
    with pytest.raises(ValueError, match="dataset identity"):
        aggregate_case_documents(
            manifest, manifest_sha, "pilot", seed_tamper, evidence_root=tmp_path
        )

    replay_tamper = copy.deepcopy(cases)
    replay_tamper[0]["worker_replay"] = None
    with pytest.raises(ValueError, match="worker replay"):
        aggregate_case_documents(
            manifest, manifest_sha, "pilot", replay_tamper, evidence_root=tmp_path
        )

    runtime_tamper = copy.deepcopy(cases)
    runtime_tamper[0]["environment"]["scipy"] = "0.0.0-tampered"
    with pytest.raises(ValueError, match="aggregate scientific runtime differs"):
        aggregate_case_documents(
            manifest, manifest_sha, "pilot", runtime_tamper, evidence_root=tmp_path
        )

    retained_tamper = copy.deepcopy(cases)
    retained_tamper[1]["retained_full_audit_results"][0]["sha256"] = "0" * 64
    with pytest.raises(ValueError, match="retained audit path/hash"):
        aggregate_case_documents(
            manifest, manifest_sha, "pilot", retained_tamper, evidence_root=tmp_path
        )

    for field, value in (
        ("scientific_result_sha256", "0" * 64),
        ("data_sha256", "0" * 64),
        ("source_dataset_fingerprint", "0" * 64),
        ("compact_evidence_typed_sha256", "0" * 64),
        ("reasons", ["synthetic"]),
    ):
        replay_auth_tamper = copy.deepcopy(cases)
        replay_auth_tamper[0]["worker_replay"]["digest_authentication"][field] = value
        with pytest.raises(ValueError, match="replay"):
            aggregate_case_documents(
                manifest,
                manifest_sha,
                "pilot",
                replay_auth_tamper,
                evidence_root=tmp_path,
            )
    replay_checks_tamper = copy.deepcopy(cases)
    replay_checks_tamper[0]["worker_replay"]["digest_authentication"]["checks"][
        "bootstrap_source_rows"
    ] = 500
    with pytest.raises(ValueError, match="replay"):
        aggregate_case_documents(
            manifest,
            manifest_sha,
            "pilot",
            replay_checks_tamper,
            evidence_root=tmp_path,
        )


def test_aggregate_rejects_compact_auth_ledger_oracle_method_and_coverage_tamper(
    tmp_path: Path,
) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    manifest["binary_binding"]["sha256"] = "a" * 64
    manifest_sha = "b" * 64
    cases = [
        _synthetic_pilot_case(manifest, manifest_sha, shard, tmp_path)
        for shard in plan_shards(manifest, "pilot")
    ]

    def rejected(name: str, mutate, *, refresh_compact: bool = False) -> None:
        tampered = copy.deepcopy(cases)
        receipt = tampered[0]["dataset_receipts"][0]
        mutate(receipt)
        if refresh_compact and receipt["digest_authentication"] is not None:
            _refresh_compact_authentication(receipt)
        try:
            aggregate_case_documents(
                manifest, manifest_sha, "pilot", tampered, evidence_root=tmp_path
            )
        except (ValueError, ValidationError):
            return
        pytest.fail(f"aggregate accepted adversarial compact mutation: {name}")

    auth_mutations = [
        ("unauthenticated_synthetic", lambda row: row.__setitem__("digest_authentication", None)),
        ("auth_scientific", lambda row: row["digest_authentication"].__setitem__("scientific_result_sha256", "0" * 64)),
        ("auth_data", lambda row: row["digest_authentication"].__setitem__("data_sha256", "0" * 64)),
        ("auth_source", lambda row: row["digest_authentication"].__setitem__("source_dataset_fingerprint", "0" * 64)),
        ("auth_checks", lambda row: row["digest_authentication"]["checks"].__setitem__("bootstrap_source_rows", 500)),
        ("auth_compact", lambda row: row["digest_authentication"].__setitem__("compact_evidence_typed_sha256", "0" * 64)),
        ("auth_kind", lambda row: row["digest_authentication"].__setitem__("kind", "synthetic")),
        ("auth_method", lambda row: row["digest_authentication"].__setitem__("method", "synthetic")),
        ("auth_status", lambda row: row["digest_authentication"].__setitem__("status", "rejected")),
        ("auth_reasons", lambda row: row["digest_authentication"].__setitem__("reasons", ["synthetic"])),
    ]
    for name, mutate in auth_mutations:
        rejected(name, mutate)

    ledger_mutations = [
        ("requested_b", lambda row: row["ledger"].__setitem__("requested_replicates", 500)),
        ("point_partition", lambda row: row["ledger"].__setitem__("point_successes", 4999)),
        ("point_failure_total", lambda row: row["ledger"].__setitem__("point_failures", 1)),
        ("point_failure_map", lambda row: row["ledger"]["point_failure_counts_by_kind"].__setitem__("non_convergence", 1)),
        ("studentized_partition", lambda row: row["ledger"].__setitem__("studentized_usable", 4999)),
        ("standard_error_unavailable_total", lambda row: row["ledger"].__setitem__("standard_error_unavailable", 1)),
        ("se_failure_map", lambda row: row["ledger"]["standard_error_unavailable_counts_by_reason"].__setitem__("singular_information", 1)),
        ("delete_partition", lambda row: row["ledger"].__setitem__("delete_one_successes", 149)),
        ("delete_failure_total", lambda row: row["ledger"].__setitem__("delete_one_failures", 1)),
        ("delete_failure_map", lambda row: row["ledger"]["delete_one_failure_counts_by_kind"].__setitem__("numerical_failure", 1)),
    ]
    for name, mutate in ledger_mutations:
        rejected(name, mutate, refresh_compact=True)

    oracle_mutations = [
        ("oracle_status", lambda row: row["oracle"].__setitem__("status", "rejected")),
        ("oracle_reasons", lambda row: row["oracle"].__setitem__("reasons", ["synthetic"])),
        ("oracle_parameter_count", lambda row: row["oracle"].__setitem__("parameter_count", 13)),
        ("oracle_requested_b", lambda row: row["oracle"].__setitem__("requested_replicates", 500)),
        ("oracle_point_successes", lambda row: row["oracle"].__setitem__("point_successes", 4999)),
        ("oracle_point_failures", lambda row: row["oracle"].__setitem__("point_failures", 1)),
        ("oracle_studentized", lambda row: row["oracle"].__setitem__("studentized_usable", 4999)),
        ("oracle_delete_successes", lambda row: row["oracle"].__setitem__("delete_one_successes", 149)),
        ("oracle_delete_failures", lambda row: row["oracle"].__setitem__("delete_one_failures", 1)),
        ("oracle_base_status", lambda row: row["oracle"].__setitem__("base_inference_available", False)),
        ("oracle_studentized_status", lambda row: row["oracle"].__setitem__("studentized_inference_available", False)),
        ("oracle_bca_status", lambda row: row["oracle"].__setitem__("bca_inference_available", False)),
        ("oracle_endpoint_checks", lambda row: row["oracle"].__setitem__("endpoint_checks", row["oracle"]["endpoint_checks"] - 1)),
        ("oracle_arithmetic_checks", lambda row: row["oracle"].__setitem__("arithmetic_checks", row["oracle"]["arithmetic_checks"] - 1)),
    ]
    for name, mutate in oracle_mutations:
        rejected(name, mutate)

    def swap_methods(row: dict) -> None:
        row["methods"][0], row["methods"][1] = row["methods"][1], row["methods"][0]

    def swap_parameters(row: dict) -> None:
        parameters = row["methods"][0]["parameters"]
        parameters[0], parameters[1] = parameters[1], parameters[0]

    def change_parameter_id(row: dict) -> None:
        row["methods"][0]["parameters"][0]["parameter_id"] = "synthetic_parameter"

    def change_method_available(row: dict) -> None:
        row["methods"][0]["available"] = False

    def unavailable_with_endpoints(row: dict) -> None:
        row["methods"][0]["parameters"][0]["available"] = False

    def reverse_endpoints(row: dict) -> None:
        parameter = row["methods"][0]["parameters"][0]
        parameter["interval_lower"], parameter["interval_upper"] = 2.0, 1.0

    def null_available_endpoint(row: dict) -> None:
        row["methods"][0]["parameters"][0]["interval_lower"] = None

    def coherent_unavailable_percentile(row: dict) -> None:
        parameter = row["methods"][0]["parameters"][0]
        parameter.update({
            "available": False,
            "interval_lower": None,
            "interval_upper": None,
            "covered_unconditional": False,
            "covered_conditional": None,
        })
        row["methods"][0]["available"] = False

    semantic_method_mutations = [
        ("method_order", swap_methods),
        ("parameter_order", swap_parameters),
        ("parameter_id", change_parameter_id),
        ("method_availability", change_method_available),
        ("parameter_availability", unavailable_with_endpoints),
        ("reversed_endpoints", reverse_endpoints),
        ("null_available_endpoint", null_available_endpoint),
        ("coherent_unavailable_percentile", coherent_unavailable_percentile),
    ]
    for name, mutate in semantic_method_mutations:
        rejected(name, mutate, refresh_compact=True)

    rejected(
        "endpoint_without_reauthentication",
        lambda row: row["methods"][0]["parameters"][0].__setitem__(
            "interval_lower", row["methods"][0]["parameters"][0]["interval_lower"] - 0.01
        ),
    )
    rejected(
        "covered_unconditional",
        lambda row: row["methods"][0]["parameters"][0].__setitem__(
            "covered_unconditional", False
        ),
    )
    rejected(
        "covered_conditional",
        lambda row: row["methods"][0]["parameters"][0].__setitem__(
            "covered_conditional", False
        ),
    )
    rejected(
        "unavailable_covered_unconditional",
        lambda row: row["methods"][2]["parameters"][0].__setitem__(
            "covered_unconditional", True
        ),
    )
    rejected(
        "unavailable_covered_conditional",
        lambda row: row["methods"][2]["parameters"][0].__setitem__(
            "covered_conditional", True
        ),
    )


def test_compact_conformance_receipt_rejects_any_coverage_flag(tmp_path: Path) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    dgp = manifest["dgps"][0]
    shard = plan_shards(manifest, "pilot")[0]
    case = _synthetic_pilot_case(manifest, "b" * 64, shard, tmp_path)
    receipt = copy.deepcopy(case["dataset_receipts"][0])
    ledger = receipt["ledger"]
    ledger.update({
        "requested_replicates": 500,
        "point_successes": 500,
        "point_failures": 0,
        "studentized_usable": 500,
        "standard_error_unavailable": 0,
    })
    for method in receipt["methods"]:
        method["available"] = False
        for parameter in method["parameters"]:
            parameter.update({
                "available": False,
                "interval_lower": None,
                "interval_upper": None,
                "covered_unconditional": None,
                "covered_conditional": None,
            })
    receipt["oracle"].update({
        "requested_replicates": 500,
        "point_successes": 500,
        "point_failures": 0,
        "studentized_usable": 500,
        "base_inference_available": False,
        "studentized_inference_available": False,
        "bca_inference_available": False,
        "endpoint_checks": 0,
        "arithmetic_checks": 0,
    })
    receipt["digest_authentication"]["checks"] = coverage_module._expected_digest_checks(
        500, dgp["sample_size"]
    )
    _refresh_compact_authentication(receipt)
    coverage_module._validate_compact_dataset_evidence(receipt, manifest, "conformance", dgp, 500)
    receipt["methods"][0]["parameters"][0]["covered_unconditional"] = False
    with pytest.raises(ValueError, match="coverage flags"):
        coverage_module._validate_compact_dataset_evidence(
            receipt, manifest, "conformance", dgp, 500
        )


def test_rejected_pilot_aggregate_is_schema_valid_and_never_enters_analytics(
    tmp_path: Path,
) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    manifest["binary_binding"]["sha256"] = "a" * 64
    manifest_sha = "b" * 64
    cases = [
        _synthetic_pilot_case(manifest, manifest_sha, shard, tmp_path)
        for shard in plan_shards(manifest, "pilot")
    ]
    rejected_case = cases[1]
    rejected_receipt = rejected_case["dataset_receipts"][0]
    rejected_receipt["status"] = "rejected"
    rejected_receipt["digest_authentication"] = None
    rejected_receipt["integrity_reasons"] = ["fixture_integrity_failure"]
    rejected_receipt["ledger"] = None
    rejected_receipt["methods"] = []
    rejected_receipt["oracle"] = None
    rejected_case["status"] = "rejected_validation_shard"

    report = aggregate_case_documents(
        manifest, manifest_sha, "pilot", cases, evidence_root=tmp_path
    )
    Draft202012Validator(json.loads(REPORT_SCHEMA.read_text(encoding="utf-8"))).validate(
        report
    )
    assert report["status"] == "pilot_evidence_rejected_product_qualification_blocked"
    assert not report["all_predeclared_evidence_accepted"]
    assert report["primary_cells"] == []
    assert report["availability_cells"] == []
    assert report["failure_summary"] == []
    assert report["rejection_diagnostics"][0]["rejected_datasets"][0][
        "integrity_reasons"
    ] == ["fixture_integrity_failure"]


def test_report_schema_enforces_accepted_pilot_exact_cardinality(tmp_path: Path) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    manifest["binary_binding"]["sha256"] = "a" * 64
    manifest_sha = "b" * 64
    cases = [
        _synthetic_pilot_case(manifest, manifest_sha, shard, tmp_path)
        for shard in plan_shards(manifest, "pilot")
    ]
    report = aggregate_case_documents(
        manifest, manifest_sha, "pilot", cases, evidence_root=tmp_path
    )
    validator = Draft202012Validator(json.loads(REPORT_SCHEMA.read_text(encoding="utf-8")))
    validator.validate(report)
    report["primary_cells"].pop()
    with pytest.raises(ValidationError):
        validator.validate(report)


def test_replay_rejects_child_that_does_not_declare_w1(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    shard = plan_shards(manifest, "conformance")[0]
    seeds = _dataset_seeds(manifest, shard["dgp_id"], "conformance", 0)
    dgp = manifest["dgps"][0]
    data_sha = hashlib.sha256(generate_dataset_csv(dgp, seeds["data"])).hexdigest()
    primary_sha = "a" * 64

    def fake_execute(command: list[str], _timeout: int) -> subprocess.CompletedProcess[str]:
        output = Path(command[command.index("--output") + 1])
        output.write_text(
            json.dumps(
                {
                    "case": {"workers": 12},
                    "scientific_result_sha256": primary_sha,
                }
            ),
            encoding="utf-8",
        )
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr(coverage_module, "execute_engine", fake_execute)
    receipt, raw_path = coverage_module.run_replay(
        Path("unused.exe"),
        manifest,
        shard,
        {
            "dataset_index": 0,
            "seeds": seeds,
            "data_sha256": data_sha,
            "scientific_result_sha256": primary_sha,
        },
        tmp_path,
    )
    assert receipt["status"] == "rejected"
    assert any("W=1" in reason for reason in receipt["reasons"])
    assert raw_path is not None and raw_path.is_file()


def test_cancellation_rejects_negative_latency_and_observed_orphan(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    shard = plan_shards(manifest, "conformance")[0]
    dgp = manifest["dgps"][0]
    seeds = _dataset_seeds(manifest, dgp["dgp_id"], "conformance", 0)
    data_sha = hashlib.sha256(generate_dataset_csv(dgp, seeds["data"])).hexdigest()

    def fake_observed(
        command: list[str], _timeout: int
    ) -> tuple[subprocess.CompletedProcess[str], int, bool]:
        output = Path(command[command.index("--output") + 1])
        output.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "kind": "cbsem_exact_cfa_interval_coverage_engine_case_v1",
                    "status": "cancelled_as_requested",
                    "qualification_status": "validation_only_product_qualification_blocked",
                    "case": {
                        "dgp_id": dgp["dgp_id"],
                        "dataset_index": 0,
                        "n_complete_cases": 150,
                        "v_observed_variables": 3,
                        "p_free_parameter_rows": 6,
                        "d_optimizer_dimensions": 6,
                        "requested_replicates": 500,
                        "workers": 12,
                        "bootstrap_seed": seeds["bootstrap"],
                        "cancel_phase": "bootstrap",
                        "cancel_after": 10,
                    },
                    "elapsed_seconds": 0.2,
                    "cancellation": {
                        "phase": "bootstrap",
                        "terminal_latency_seconds": -0.1,
                    },
                    "fixture": {"data_sha256": data_sha},
                }
            ),
            encoding="utf-8",
        )
        return subprocess.CompletedProcess(command, 0, "", ""), 1, False

    monkeypatch.setattr(coverage_module, "execute_engine_observed", fake_observed)
    receipt, raw_path = coverage_module.run_cancellation_probe(
        Path("unused.exe"), manifest, shard, tmp_path
    )
    assert receipt["status"] == "rejected"
    assert receipt["orphan_processes"] == 1
    assert "cancellation_receipt_or_latency_differs" in receipt["reasons"]
    assert "cancellation_orphan_process_limit_exceeded" in receipt["reasons"]
    assert raw_path is not None and raw_path.is_file()


def test_strict_schemas_reject_unknown_fields_and_never_allow_qualification() -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    manifest_schema = json.loads(MANIFEST_SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(manifest_schema)
    Draft202012Validator(manifest_schema).validate(manifest)
    altered = copy.deepcopy(manifest)
    altered["qualification_boundary"]["status"] = "qualified"
    with pytest.raises(ValidationError):
        Draft202012Validator(manifest_schema).validate(altered)
    altered = copy.deepcopy(manifest)
    altered["unexpected"] = True
    with pytest.raises(ValidationError):
        Draft202012Validator(manifest_schema).validate(altered)
    altered = copy.deepcopy(manifest)
    altered["scientific_runtime"]["scipy"] = "0.0.0-tampered"
    with pytest.raises(ValidationError):
        Draft202012Validator(manifest_schema).validate(altered)
    altered = copy.deepcopy(manifest)
    altered["coverage_contract"]["simultaneous_two_sided_tail"]["denominator"] = 2279
    with pytest.raises(ValidationError):
        Draft202012Validator(manifest_schema).validate(altered)
    altered = copy.deepcopy(manifest)
    altered["coverage_contract"]["binomial_endpoint_kernel"] = "uncertified"
    with pytest.raises(ValidationError):
        Draft202012Validator(manifest_schema).validate(altered)
    case_schema = json.loads(CASE_SCHEMA.read_text(encoding="utf-8"))
    environment_schema = case_schema["$defs"]["environment"]
    environment = {
        "os": "test",
        "os_release": "test",
        "architecture": "test",
        "logical_cores": 1,
        **manifest["scientific_runtime"],
    }
    Draft202012Validator(environment_schema).validate(environment)
    del environment["scipy"]
    with pytest.raises(ValidationError):
        Draft202012Validator(environment_schema).validate(environment)
    for path in (CASE_SCHEMA, REPORT_SCHEMA):
        Draft202012Validator.check_schema(json.loads(path.read_text(encoding="utf-8")))


def test_plan_and_dry_run_execute_no_workloads_or_outputs(tmp_path: Path) -> None:
    runner = ROOT / "validation/cbsem_exact_cfa_interval_coverage_v1.py"
    planned = subprocess.run(
        [sys.executable, str(runner), "--output-root", str(tmp_path), "plan", "--stage", "final"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert planned.returncode == 0, planned.stderr
    assert "immutable_plan_only_no_workloads_executed" in planned.stdout
    dry = subprocess.run(
        [
            sys.executable,
            str(runner),
            "run-shard",
            "--stage", "conformance",
            "--shard-id", "conformance-dgp_a_one_factor_n150-s00",
            "--run-id", "dry",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert dry.returncode == 0, dry.stderr
    assert "dry_run_no_workloads_or_outputs" in dry.stdout
    assert list(tmp_path.rglob("*")) == []


def _available_oracle_document() -> dict:
    parameter_id = "parameter_fixture"
    point = 1.0
    point_se = 0.2
    bootstrap_values = [0.8, 0.9, 1.1, 1.2]
    standard_errors = [0.1, 0.1, 0.1, 0.1]
    jackknife_values = [0.8, 0.95, 1.05, 1.3]
    successes = [
        {"replicate_index": index, "parameter_estimates": [value]}
        for index, value in enumerate(bootstrap_values)
    ]
    mean = float(np.mean(bootstrap_values))
    standard_error = float(np.std(bootstrap_values, ddof=1))
    pivots = [
        (value - point) / se for value, se in zip(bootstrap_values, standard_errors, strict=True)
    ]
    lower_pivot = oracle_module.type7(pivots, 0.025)
    upper_pivot = oracle_module.type7(pivots, 0.975)
    bca_outcome = oracle_module._bca(bootstrap_values, point, jackknife_values)
    assert bca_outcome["status"] == "available"
    return {
        "status": "completed",
        "truth": [{"parameter_id": parameter_id}],
        "original": {
            "refit": {"free_parameters": [{"parameter_id": parameter_id, "estimate": point}]}
        },
        "bootstrap": {
            "base": {
                "parameter_ids": [parameter_id],
                "requested_replicates": 4,
                "minimum_usable_replicates": 2,
                "successful_refits": successes,
                "failed_refits": [],
                "inference": {"status": "available"},
                "intervals": [
                    {
                        "parameter_id": parameter_id,
                        "original": point,
                        "bootstrap_mean": mean,
                        "bias": mean - point,
                        "standard_error": standard_error,
                        "percentile_lower": oracle_module.type7(bootstrap_values, 0.025),
                        "percentile_upper": oracle_module.type7(bootstrap_values, 0.975),
                    }
                ],
            },
            "studentized": {
                "parameter_ids": [parameter_id],
                "point_standard_errors": {
                    "outcome": {
                        "status": "available",
                        "parameters": [
                            {"parameter_id": parameter_id, "standard_error": point_se}
                        ],
                    }
                },
                "refit_standard_errors": [
                    {
                        "replicate_index": index,
                        "outcome": {"status": "available", "standard_errors": [se]},
                    }
                    for index, se in enumerate(standard_errors)
                ],
                "studentized_usable_replicates": 4,
                "inference": {"status": "available"},
                "intervals": [
                    {
                        "parameter_id": parameter_id,
                        "outcome": {
                            "status": "available",
                            "point_estimate": point,
                            "point_standard_error": point_se,
                            "lower_pivot_quantile": lower_pivot,
                            "upper_pivot_quantile": upper_pivot,
                            "interval_lower": point - upper_pivot * point_se,
                            "interval_upper": point - lower_pivot * point_se,
                        },
                    }
                ],
            },
        },
        "bca": {
            "parameter_ids": [parameter_id],
            "delete_one_case_count": 4,
            "successful_delete_one_refits": [
                {
                    "omitted_complete_case_position": index,
                    "parameter_estimates": [value],
                }
                for index, value in enumerate(jackknife_values)
            ],
            "failed_delete_one_refits": [],
            "inference": {"status": "available"},
            "intervals": [{"parameter_id": parameter_id, "outcome": bca_outcome}],
        },
    }


def _bounds_close(left: tuple[float, float], right: tuple[float, float]) -> bool:
    return left == pytest.approx(right, rel=1e-10, abs=1e-12)


def _digest_authentication(
    document: dict,
    data_sha: str,
    source_fingerprint: str,
    replicates: int,
    n: int,
    *,
    ledger: dict | None = None,
    methods: list[dict] | None = None,
) -> dict:
    def rust_style_counts(rows: list[dict], field: str, allowed: set[str]) -> dict[str, int]:
        counts = {value: 0 for value in sorted(allowed)}
        for row in rows:
            if row.get(field) in counts:
                counts[row[field]] += 1
        return counts

    if ledger is None or methods is None:
        base = document["bootstrap"]["base"]
        studentized = document["bootstrap"]["studentized"]
        bca = document["bca"]
        successes = base["successful_refits"]
        failures = base["failed_refits"]
        se_receipts = studentized["refit_standard_errors"]
        unavailable = [
            row["outcome"]
            for row in se_receipts
            if row["outcome"]["status"] == "unavailable"
        ]
        delete_successes = bca["successful_delete_one_refits"]
        delete_failures = bca["failed_delete_one_refits"]
        ledger = {
            "requested_replicates": replicates,
            "point_successes": len(successes),
            "point_failures": len(failures),
            "point_failure_counts_by_kind": rust_style_counts(
                failures, "kind", coverage_module.POINT_FAILURE_KINDS
            ),
            "studentized_usable": len(successes) - len(unavailable),
            "standard_error_unavailable": len(unavailable),
            "standard_error_unavailable_counts_by_reason": rust_style_counts(
                unavailable, "reason", coverage_module.SE_UNAVAILABLE_REASONS
            ),
            "delete_one_successes": len(delete_successes),
            "delete_one_failures": len(delete_failures),
            "delete_one_failure_counts_by_kind": rust_style_counts(
                delete_failures, "kind", coverage_module.POINT_FAILURE_KINDS
            ),
            "failure_rate_inference_unit": "dataset_not_bootstrap_replicate",
        }
        truth_by_id = {row["parameter_id"]: row for row in document["truth"]}
        methods = coverage_module._method_rows(
            document, truth_by_id, evaluate_coverage=False
        )
    compact = compact_evidence_payload(
        document["scientific_result_sha256"],
        data_sha,
        source_fingerprint,
        ledger,
        methods,
    )
    return {
        "schema_version": 1,
        "kind": "cbsem_exact_cfa_interval_coverage_digest_verification_receipt_v1",
        "status": "accepted",
        "method": "bound_rust_exact_schedule_and_source_verifier_v1",
        "document_typed_sha256": typed_json_sha256(document),
        "scientific_result_sha256": document["scientific_result_sha256"],
        "compact_evidence_typed_sha256": typed_json_sha256(compact),
        "data_sha256": data_sha,
        "source_dataset_fingerprint": source_fingerprint,
        "checks": {
            "scientific_result": 1,
            "base_point_result": 1,
            "source_dataset_fingerprint": 1,
            "compiler_analytical_identity": 1,
            "plan": 1,
            "model_scientific_identity": 1,
            "outer_recipe_analytical_identity": 1,
            "complete_case_universe": 1,
            "original_sampling_positions": 1,
            "original_source_rows": 1,
            "bootstrap_schedule_positions": replicates,
            "bootstrap_source_rows": replicates,
            "delete_one_sampling_positions": n,
            "delete_one_source_rows": n,
        },
        "reasons": [],
    }


def _refresh_scientific_authentication(
    document: dict, data_sha: str, source_fingerprint: str, replicates: int, n: int
) -> dict:
    document["scientific_result_sha256"] = typed_json_sha256(
        {
            "original": document["original"],
            "bootstrap": document["bootstrap"],
            "bca": document["bca"],
        }
    )
    return _digest_authentication(document, data_sha, source_fingerprint, replicates, n)


def _refresh_compact_authentication(receipt: dict) -> None:
    authentication = receipt["digest_authentication"]
    compact = compact_evidence_payload(
        receipt["scientific_result_sha256"],
        receipt["data_sha256"],
        authentication["source_dataset_fingerprint"],
        receipt["ledger"],
        receipt["methods"],
    )
    authentication["compact_evidence_typed_sha256"] = typed_json_sha256(compact)


def _derive_schedule_rows(manifest: dict, n: int, source_fingerprint: str, seed: int) -> list[dict]:
    binary = ROOT / manifest["binary_binding"]["path"]
    process = subprocess.run(
        [
            str(binary),
            "--derive-schedule-digests", str(n),
            "--source-fingerprint", source_fingerprint,
            "--seed", str(seed),
            "--replicates", "500",
            "--stream-token", "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert process.returncode == 0, process.stderr
    receipt = json.loads(process.stdout)
    assert receipt["kind"] == "cbsem_exact_cfa_interval_coverage_schedule_digest_plan_v1"
    assert len(receipt["rows"]) == 500
    return receipt["rows"]


def _conformance_document(manifest: dict) -> tuple[dict, dict, int, str, dict]:
    dgp = manifest["dgps"][0]
    seeds = {
        purpose: derive_seed(
            manifest["seed_contract"][f"{purpose}_master_seed"],
            dgp["dgp_id"], "conformance", 0, purpose,
        )
        for purpose in ("data", "bootstrap", "oracle")
    }
    data_sha = hashlib.sha256(generate_dataset_csv(dgp, seeds["data"])).hexdigest()
    parameter_ids = [row["parameter_id"] for row in dgp["parameter_truth"]]
    estimates = [row["true_value"] for row in dgp["parameter_truth"]]
    source_fingerprint = "1" * 64
    schedule_rows = _derive_schedule_rows(
        manifest, dgp["sample_size"], source_fingerprint, seeds["bootstrap"]
    )
    successes = [
        {
            "replicate_index": index,
            "sampling_positions_sha256": schedule_rows[index]["sampling_positions_sha256"],
            "sample_indices_sha256": schedule_rows[index]["sample_indices_sha256"],
            "parameter_estimates": [value + (index % 7 - 3) * 1e-4 for value in estimates],
            "iterations": 1,
            "objective": 0.0,
            "gradient_norm": 0.0,
        }
        for index in range(500)
    ]
    se_receipts = [
        {
            "replicate_index": index,
            "outcome": {"status": "available", "information_method": "cbsem_ml_expected_information_delta_method_v1", "standard_errors": [0.1] * 6},
        }
        for index in range(500)
    ]
    delete_successes = [
        {
            "omitted_complete_case_position": index,
            "omitted_source_row_index": index,
            "retained_sampling_positions_sha256": sampling_positions_sha256(
                150, [position for position in range(150) if position != index]
            ),
            "retained_sample_indices_sha256": source_rows_sha256(
                source_fingerprint,
                150,
                [position for position in range(150) if position != index],
            ),
            "parameter_estimates": [value + (index % 5 - 2) * 1e-4 for value in estimates],
            "iterations": 1,
            "objective": 0.0,
            "gradient_norm": 0.0,
        }
        for index in range(150)
    ]
    document = {
        "schema_version": 1,
        "kind": "cbsem_exact_cfa_interval_coverage_engine_case_v1",
        "status": "completed",
        "qualification_status": "validation_only_product_qualification_blocked",
        "digest_contract": copy.deepcopy(manifest["digest_contract"]),
        "case": {
            "dgp_id": dgp["dgp_id"], "dataset_index": 0,
            "n_complete_cases": 150, "v_observed_variables": 3,
            "p_free_parameter_rows": 6, "d_optimizer_dimensions": 6,
            "requested_replicates": 500, "workers": 12,
            "bootstrap_seed": seeds["bootstrap"], "cancel_phase": None, "cancel_after": None,
        },
        "fixture": {"data_sha256": data_sha},
        "truth": copy.deepcopy(dgp["parameter_truth"]),
        "scientific_result_sha256": "0" * 64,
        "original": {
            "refit": {
                "method_version": "cbsem_exact_case_bootstrap_v1",
                "estimator_method_version": "cbsem_ml_exact_parameter_table_v3",
                "source_dataset_id": _expected_dataset_uuid(dgp["dgp_id"], 0),
                "source_dataset_fingerprint": source_fingerprint,
                "compiler_analytical_identity_sha256": "2" * 64,
                "plan_sha256": "3" * 64,
                "model_scientific_sha256": "4" * 64,
                "source_row_count": 150,
                "complete_case_sample_size": 150,
                "complete_case_universe_digest_method": "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1",
                "complete_case_universe_sha256": complete_case_universe_sha256(
                    source_fingerprint, 150, list(range(150))
                ),
                "resampled_observations": 150,
                "covariance_denominator": "maximum_likelihood_n",
                "sample_indices_digest_method": "sha256_source_fingerprint_and_ordered_u64_indices_v1",
                "sampling_positions_digest_method": "sha256_complete_case_n_and_ordered_sampling_positions_v1",
                "sampling_positions_sha256": sampling_positions_sha256(150, list(range(150))),
                "sample_indices_sha256": source_rows_sha256(
                    source_fingerprint, 150, list(range(150))
                ),
                "free_parameters": [{"parameter_id": parameter_id, "estimate": value} for parameter_id, value in zip(parameter_ids, estimates, strict=True)],
                "iterations": 1,
                "objective": 0.0,
                "gradient_norm": 0.0,
            },
            "standard_errors": {
                "method_version": "cbsem_exact_case_bootstrap_refit_standard_errors_v1",
                "outcome": {
                    "status": "available",
                    "information_method": "cbsem_ml_expected_information_delta_method_v1",
                    "parameters": [
                        {"parameter_id": parameter_id, "standard_error": 0.1}
                        for parameter_id in parameter_ids
                    ],
                },
            },
        },
        "bootstrap": {
            "base": {
                "method_version": "cbsem_exact_case_bootstrap_v1",
                "estimator_method_version": "cbsem_ml_exact_parameter_table_v3",
                "source_dataset_id": _expected_dataset_uuid(dgp["dgp_id"], 0),
                "source_dataset_fingerprint": source_fingerprint,
                "outer_recipe_analytical_identity_sha256": "8" * 64,
                "base_point_result_sha256": "0" * 64,
                "compiler_analytical_identity_sha256": "2" * 64,
                "plan_sha256": "3" * 64,
                "model_scientific_sha256": "4" * 64,
                "complete_case_sample_size": 150,
                "complete_case_universe_digest_method": "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1",
                "complete_case_universe_sha256": complete_case_universe_sha256(
                    source_fingerprint, 150, list(range(150))
                ),
                "covariance_denominator": "maximum_likelihood_n",
                "sample_indices_digest_method": "sha256_source_fingerprint_and_ordered_u64_indices_v1",
                "sampling_positions_digest_method": "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1",
                "interval_method": "percentile_type7_v1",
                "confidence_level": 0.95,
                "parameter_ids": parameter_ids, "requested_replicates": 500, "attempted_refits": 500,
                "usable_replicates": 500, "failed_replicates": 0,
                "minimum_usable_fraction": 0.9, "minimum_usable_replicates": 1000,
                "seed": seeds["bootstrap"], "stream_token": "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1",
                "retry_policy": "no_retry_fixed_preplanned_primary_draws_v1", "max_attempts_per_replicate": 1,
                "inference": {"status": "unavailable", "reason_code": "insufficient_usable_refits", "message": "fixture"},
                "intervals": [], "successful_refits": successes, "failed_refits": [],
            },
            "studentized": {
                "method_version": "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1",
                "standard_error_method_version": "cbsem_exact_case_bootstrap_refit_standard_errors_v1",
                "expected_information_method": "cbsem_ml_expected_information_delta_method_v1",
                "pivot_method": "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1",
                "quantile_method": "percentile_type7_v1",
                "interval_method": "reversed_type7_studentized_pivot_v1",
                "archive_validation_scope": "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1",
                "confidence_level": 0.95,
                "parameter_ids": parameter_ids, "minimum_usable_replicates": 1000,
                "minimum_usable_fraction": 0.9,
                "studentized_usable_replicates": 500,
                "point_standard_errors": {
                    "method_version": "cbsem_exact_case_bootstrap_refit_standard_errors_v1",
                    "outcome": {
                        "status": "available",
                        "information_method": "cbsem_ml_expected_information_delta_method_v1",
                        "parameters": [
                            {"parameter_id": parameter_id, "standard_error": 0.1}
                            for parameter_id in parameter_ids
                        ],
                    },
                },
                "inference": {"status": "unavailable", "reason": "insufficient_studentized_usable_replicates", "message": "fixture"},
                "intervals": [{"parameter_id": parameter_id, "outcome": {"status": "unavailable", "reason": "insufficient_studentized_usable_replicates"}} for parameter_id in parameter_ids],
                "refit_standard_errors": se_receipts,
            },
        },
        "bca": {
            "method_version": "cbsem_exact_case_bootstrap_bca_interval_v1",
            "base_bootstrap_method_version": "cbsem_exact_case_bootstrap_v1",
            "outer_recipe_analytical_identity_sha256": "8" * 64,
            "base_point_result_sha256": "0" * 64,
            "compiler_analytical_identity_sha256": "2" * 64,
            "plan_sha256": "3" * 64,
            "model_scientific_sha256": "4" * 64,
            "delete_one_refit_method_version": "cbsem_exact_case_bootstrap_delete_one_refit_v1",
            "bias_correction_method": "midrank_less_plus_half_ties_no_clamp_v1",
            "acceleration_method": "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2",
            "adjusted_probability_method": "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2",
            "quantile_method": "percentile_type7_v1",
            "retry_policy": "no_retry_exactly_one_fit_per_omitted_case_v1",
            "confidence_level": 0.95,
            "bootstrap_usable_replicates": 500,
            "minimum_bootstrap_usable_replicates": 1000,
            "parameter_ids": parameter_ids, "delete_one_case_count": 150,
            "inference": {"status": "unavailable", "reason": "base_inference_unavailable", "message": "fixture"},
            "intervals": [{"parameter_id": parameter_id, "outcome": {"status": "unavailable", "reason": "base_inference_unavailable"}} for parameter_id in parameter_ids],
            "successful_delete_one_refits": delete_successes, "failed_delete_one_refits": [],
        },
    }
    base_point_sha = typed_json_sha256(document["original"]["refit"])
    document["bootstrap"]["base"]["base_point_result_sha256"] = base_point_sha
    document["bca"]["base_point_result_sha256"] = base_point_sha
    document["scientific_result_sha256"] = typed_json_sha256(
        {
            "original": document["original"],
            "bootstrap": document["bootstrap"],
            "bca": document["bca"],
        }
    )
    authentication = _digest_authentication(
        document, data_sha, source_fingerprint, 500, dgp["sample_size"]
    )
    return document, dgp, seeds["bootstrap"], data_sha, authentication


def _synthetic_pilot_case(
    manifest: dict, manifest_sha: str, shard: dict, evidence_root: Path
) -> dict:
    dgp = next(row for row in manifest["dgps"] if row["dgp_id"] == shard["dgp_id"])
    receipts = []
    for index in shard["dataset_indices"]:
        seeds = _dataset_seeds(manifest, dgp["dgp_id"], "pilot", index)
        data_sha = hashlib.sha256(generate_dataset_csv(dgp, seeds["data"])).hexdigest()
        scientific_sha = typed_json_sha256(
            {"dgp_id": dgp["dgp_id"], "stage": "pilot", "dataset_index": index}
        )
        methods = []
        for method in ("percentile", "studentized", "bca"):
            parameters = []
            for parameter_ordinal, parameter in enumerate(dgp["parameter_truth"]):
                unavailable = (
                    dgp["dgp_id"] == "dgp_a_one_factor_n150"
                    and index == 0
                    and method == "bca"
                    and parameter_ordinal == 0
                )
                parameters.append({
                    "parameter_id": parameter["parameter_id"],
                    "available": not unavailable,
                    "interval_lower": None if unavailable else parameter["true_value"] - 0.1,
                    "interval_upper": None if unavailable else parameter["true_value"] + 0.1,
                    "covered_unconditional": not unavailable,
                    "covered_conditional": None if unavailable else True,
                })
            methods.append({"method": method, "available": all(row["available"] for row in parameters), "parameters": parameters})
        ledger = {
            "requested_replicates": 5000, "point_successes": 5000, "point_failures": 0,
            "point_failure_counts_by_kind": {kind: 0 for kind in ("moment_matrix_not_positive_definite", "non_convergence", "inadmissible_solution", "numerical_failure")},
            "studentized_usable": 5000, "standard_error_unavailable": 0,
            "standard_error_unavailable_counts_by_reason": {reason: 0 for reason in ("singular_information", "information_not_positive_definite", "invalid_information_variance_or_standard_error", "derivative_unavailable", "numerical_information_failure")},
            "delete_one_successes": dgp["sample_size"], "delete_one_failures": 0,
            "delete_one_failure_counts_by_kind": {kind: 0 for kind in ("moment_matrix_not_positive_definite", "non_convergence", "inadmissible_solution", "numerical_failure")},
            "failure_rate_inference_unit": "dataset_not_bootstrap_replicate",
        }
        authentication = _digest_authentication(
            {"scientific_result_sha256": scientific_sha},
            data_sha,
            "1" * 64,
            5000,
            dgp["sample_size"],
            ledger=ledger,
            methods=methods,
        )
        available_counts = {
            row["method"]: sum(parameter["available"] for parameter in row["parameters"])
            for row in methods
        }
        receipts.append({
            "dataset_index": index, "status": "accepted", "manifest_sha256": manifest_sha,
            "dgp_id": dgp["dgp_id"], "stage": "pilot", "seeds": seeds,
            "data_sha256": data_sha, "scientific_result_sha256": scientific_sha,
            "digest_authentication": authentication,
            "integrity_reasons": [], "coverage_evaluated": True,
            "ledger": ledger,
            "methods": methods,
            "oracle": {
                "schema_version": 1, "kind": "cbsem_exact_cfa_interval_coverage_oracle_receipt_v1", "status": "accepted",
                "parameter_count": len(dgp["parameter_truth"]), "requested_replicates": 5000, "point_successes": 5000, "point_failures": 0,
                "studentized_usable": 5000, "delete_one_successes": dgp["sample_size"], "delete_one_failures": 0,
                "base_inference_available": True, "studentized_inference_available": True, "bca_inference_available": True,
                "endpoint_checks": 2 * sum(available_counts.values()),
                "arithmetic_checks": 4 * available_counts["percentile"] + 4 * available_counts["studentized"] + 5 * available_counts["bca"],
                "endpoint_relative_tolerance": 1e-12,
                "percentile_mean_validation_method": "math_fsum_reference_higham_gamma_b_minus_1_outward_binary64_v1",
                "percentile_bias_binding_method": "observed_mean_minus_point_binary64_bit_exact_v1",
                "bca_acceleration_method": "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2",
                "bca_adjusted_probability_method": "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2",
                "reasons": [],
            },
        })
    audit_path = Path("shards") / shard["shard_id"] / "audit" / f"dataset-{shard['dataset_indices'][0]:06d}.json"
    absolute_audit_path = evidence_root / audit_path
    absolute_audit_path.parent.mkdir(parents=True, exist_ok=True)
    audit_payload = json.dumps({"shard_id": shard["shard_id"]}, sort_keys=True).encode()
    absolute_audit_path.write_bytes(audit_payload)
    replay = None
    if shard["shard_ordinal"] == 0:
        first = receipts[0]
        replay = {
            "status": "accepted",
            "dataset_index": first["dataset_index"],
            "primary_workers": 12,
            "replay_workers": 1,
            "primary_scientific_result_sha256": first["scientific_result_sha256"],
            "replay_scientific_result_sha256": first["scientific_result_sha256"],
            "digest_authentication": copy.deepcopy(first["digest_authentication"]),
            "reasons": [],
        }
    return {
        "schema_version": 1, "kind": "cbsem_exact_cfa_interval_coverage_shard_case_v1", "status": "accepted_validation_shard",
        "qualification_status": "blocked_not_product_qualification", "manifest_sha256": manifest_sha, "binary_sha256": manifest["binary_binding"]["sha256"],
        "shard": shard, "environment": {"os": "test", "os_release": "test", "architecture": "test", "logical_cores": 1, **manifest["scientific_runtime"]},
        "dataset_receipts": receipts, "worker_replay": replay, "cancellation": None,
        "retained_full_audit_results": [{
            "artifact_kind": "dataset",
            "dataset_index": shard["dataset_indices"][0],
            "path": audit_path.as_posix(),
            "sha256": hashlib.sha256(audit_payload).hexdigest(),
        }],
        "retention_policy": manifest["retention_contract"],
    }
