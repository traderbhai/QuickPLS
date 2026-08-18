#!/usr/bin/env python3
"""Focused fail-closed tests for the current-source PLS qualification lane."""

from __future__ import annotations

import copy
import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import pls_algorithm_v1_current_build_receipts as build_receipts  # noqa: E402
import pls_algorithm_v1_qualification_factory as factory  # noqa: E402
from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_document,
)


class PlsAlgorithmV1QualificationFactoryTests(unittest.TestCase):
    @staticmethod
    def _write_json(path: Path, value: object) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )

    @classmethod
    def _write_artifact(
        cls, root: Path, relative: str, value: object
    ) -> dict[str, object]:
        path = root / relative
        if isinstance(value, bytes):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(value)
        else:
            cls._write_json(path, value)
        return {
            "path": path.relative_to(root).as_posix(),
            "size_bytes": path.stat().st_size,
            "sha256": factory.sha256_file(path),
        }

    @staticmethod
    def _sorted_json_value(value: object) -> object:
        return json.loads(json.dumps(value, sort_keys=True, allow_nan=False))

    @classmethod
    def _synthetic_canonical_table(
        cls,
        *,
        table_id: str,
        columns: list[tuple[str, str]],
        rows: list[tuple[str, list[object]]],
        capability_cell: dict[str, object],
    ) -> dict[str, object]:
        column_documents = []
        for column_id, data_type in columns:
            column = {
                "id": column_id,
                "label": column_id.replace("_", " ").title(),
                "data_type": data_type,
                "description": f"Synthetic {column_id}.",
                "role": "estimate" if data_type == "number" else "label",
            }
            if data_type == "number":
                column["default_precision"] = 6
            column_documents.append(column)
        return {
            "id": table_id,
            "title": table_id.replace("_", " ").title(),
            "description": f"Synthetic {table_id} table.",
            "columns": column_documents,
            "rows": [
                {
                    "id": row_id,
                    "cells": [
                        {
                            "kind": data_type,
                            "value": value,
                        }
                        for (_, data_type), value in zip(columns, values, strict=True)
                    ],
                }
                for row_id, values in rows
            ],
            "footnote_ids": [],
            "capability_cells": [capability_cell],
        }

    @classmethod
    def _synthetic_variant_documents(
        cls,
        *,
        variant: str,
        dataset_id: str,
        project_id: str,
        capability_cell: dict[str, object],
    ) -> dict[str, object]:
        index = list(factory.CURRENT_PRODUCT_VARIANTS).index(variant)
        weighting, mode = factory.CURRENT_PRODUCT_VARIANTS[variant]
        model = factory._expected_current_product_model(variant, dataset_id)  # noqa: SLF001
        sorted_model = cls._sorted_json_value(model)
        model_digest = factory._serialized_json_sha256(sorted_model)  # noqa: SLF001
        recipe_id = f"00000000-0000-0000-0000-{0x2000 + index:012x}"
        recipe = {
            "schema_version": 4,
            "id": recipe_id,
            "created_at": f"2026-08-15T00:00:0{index}Z",
            "dataset_fingerprint": factory.CURRENT_PRODUCT_DATASET_FINGERPRINT,
            "model_binding": {
                "kind": "project_sem_model_v4_reference",
                "model_id": model["id"],
                "scientific_sha256": model_digest,
            },
            "estimand_confirmation": "confirmed_composite",
            "settings": {
                "method": "pls_pm",
                "weighting_scheme": weighting,
                "tolerance": 1.0e-7,
                "max_iterations": 3000,
                "bootstrap_samples": 0,
                "studentized_inner_samples": 0,
                "permutation_samples": 0,
                "seed": 20_260_815,
                "workers": 1,
                "confidence_level": 0.95,
                "preprocessing": "standardized",
                "missing_data": "listwise_deletion",
                "case_weight_column": None,
            },
            "method_config": {"kind": "pls_algorithm"},
            "metadata": {"qualification_use": "current_product_work_evidence_only"},
            "legacy_source": {
                "source_schema_version": 3,
                "source_recipe_sha256": f"{index + 1}" * 64,
            },
        }
        request = {
            "surface": "internal_labs",
            "experimentalLabsEnabled": True,
            "residentData": "project_resident",
            "datasetId": dataset_id,
            "datasetFingerprint": factory.CURRENT_PRODUCT_DATASET_FINGERPRINT,
            "recipe": recipe,
            "model": model,
            "compilerTarget": factory.CURRENT_PRODUCT_COMPILER_TARGET,
            "capabilityCell": capability_cell,
        }
        blocks = []
        for construct, indicators in (("x", ("x1", "x2")), ("y", ("y1", "y2"))):
            blocks.append(
                {
                    "construct_id": f"construct:{construct}",
                    "mode": mode,
                    "indicators": [
                        {
                            "variable_id": f"observed:{indicator}",
                            "source_column": indicator,
                            "parameter_id": {
                                "x1": "parameter_78_7831",
                                "x2": "parameter_78_7832",
                                "y1": "parameter_79_7931",
                                "y2": "parameter_79_7932",
                            }[indicator],
                        }
                        for indicator in indicators
                    ],
                }
            )
        plan_wrapper = {
            "kind": factory.CURRENT_PRODUCT_COMPILER_TARGET,
            "plan": {
                "model_id": model["id"],
                "scientific_hash": model_digest,
                "dataset_id": dataset_id,
                "blocks": blocks,
                "paths": [
                    {
                        "relation_id": "structural_78_79",
                        "source": "construct:x",
                        "target": "construct:y",
                        "parameter_id": "regression_78_79",
                    }
                ],
            },
        }
        sorted_plan_wrapper = cls._sorted_json_value(plan_wrapper)
        receipt = {
            "schema_version": 1,
            "recipe_id": recipe_id,
            "recipe_document_sha256": "5678"[index] * 64,
            "recipe_analytical_sha256": "9abc"[index] * 64,
            "model_id": model["id"],
            "model_document_sha256": model_digest,
            "model_scientific_sha256": model_digest,
            "dataset_fingerprint": factory.CURRENT_PRODUCT_DATASET_FINGERPRINT,
            "compiler_target": factory.CURRENT_PRODUCT_COMPILER_TARGET,
            "compiler_version": factory.CURRENT_PRODUCT_COMPILER_VERSION,
            "capability_cell": capability_cell,
            "plan_sha256": factory._serialized_json_sha256(sorted_plan_wrapper),  # noqa: SLF001
            "analytical_identity_sha256": "def0"[index] * 64,
        }
        compiled = {"plan": plan_wrapper, "receipt": receipt}
        quantities = factory._expected_current_product_quantities(variant)  # noqa: SLF001
        outer_estimates = []
        for construct, indicator in (
            ("construct:x", "x1"),
            ("construct:x", "x2"),
            ("construct:y", "y1"),
            ("construct:y", "y2"),
        ):
            outer_estimates.append(
                {
                    "construct": construct,
                    "indicator": indicator,
                    "weight": quantities[("weight", construct, indicator)],
                    "loading": quantities[("loading", construct, indicator)],
                }
            )
        coefficient = quantities[("path", "construct:x", "construct:y")]
        point_estimate_attribution = {
            "contract_version": "pls_point_estimate_attribution_v1",
            "preprocessing": "standardized",
            "indicator_centering": "sample_mean",
            "indicator_scaling": "sample_standard_deviation",
            "outer_weights": "preprocessed_indicator_to_unit_variance_construct_score",
            "outer_loadings": "indicator_construct_score_correlation",
            "construct_scores": "zero_mean_unit_variance_construct_score",
            "structural_paths": "standardized_construct_score_regression",
            "effects": "standardized_structural_path_decomposition",
        }
        algorithm_convergence_receipt = None
        if factory._current_product_requires_convergence_receipt(variant):  # noqa: SLF001
            update_rule = (
                "mode_a_covariance" if mode == "mode_a" else "mode_b_ols"
            )
            algorithm_convergence_receipt = {
                "contract_version": "pls_algorithm_convergence_receipt_v1",
                "weighting_scheme": weighting,
                "maximum_iterations": 3000,
                "stop_criterion": 1.0e-7,
                "comparison": "less_than_or_equal",
                "performed_iterations": 4,
                "estimated_block_updates": 8,
                "termination_reason": "converged_tolerance",
                "final_max_outer_weight_change": 5.0e-8,
                "blocks": [
                    {
                        "construct_id": f"construct:{construct}",
                        "indicator_order": list(indicators),
                        "update_rule": update_rule,
                        "initialization": "standard_unit_weights",
                    }
                    for construct, indicators in (
                        ("x", ("x1", "x2")),
                        ("y", ("y1", "y2")),
                    )
                ],
            }
        estimation = {
            "method_version": factory.METHOD_VERSION,
            "converged": True,
            "iterations": 4,
            "point_estimate_attribution": point_estimate_attribution,
            "used_observations": 6,
            "omitted_observations": 0,
            "transforms": [
                {"indicator": indicator, "mean": 1.0, "scale": 1.0}
                for indicator in ("x1", "x2", "y1", "y2")
            ],
            "construct_scores": {
                "construct:x": [-1.0, -0.5, 0.0, 0.5, 1.0, 1.5],
                "construct:y": [-1.1, -0.4, 0.1, 0.4, 1.1, 1.4],
            },
            "outer_estimates": outer_estimates,
            "paths": [
                {
                    "source": "construct:x",
                    "target": "construct:y",
                    "coefficient": coefficient,
                }
            ],
            "control_estimates": [],
            "effects": [
                {
                    "source": "construct:x",
                    "target": "construct:y",
                    "direct": coefficient,
                    "indirect": 0.0,
                    "total": coefficient,
                }
            ],
            "mediation": {
                "method_version": "pls_mediation_v1",
                "tolerance": 1.0e-12,
                "estimates": [
                    {
                        "source": "construct:x",
                        "target": "construct:y",
                        "direct": coefficient,
                        "indirect": 0.0,
                        "total": coefficient,
                        "variance_accounted_for": 0.0,
                        "classification": "direct_only",
                        "warning": "synthetic direct-only effect",
                    }
                ],
                "warnings": ["synthetic direct-only effect"],
            },
            "r_squared": {"construct:y": coefficient * coefficient},
            "warnings": [],
        }
        if algorithm_convergence_receipt is not None:
            estimation["algorithm_convergence_receipt"] = algorithm_convergence_receipt
        for excluded in (
            "plsc",
            "endogeneity",
            "nonlinear_effects",
            "moderated_mediation",
            "cta_pls",
            "wpls",
            "cca",
            "predict",
            "segmentation",
            "mga",
            "micom",
            "mga_permutation",
            "fimix",
            "ipma",
            "cbsem",
            "pca",
            "regression",
            "nca",
            "gsca",
        ):
            estimation[excluded] = None
        execution = {
            "schema_version": 1,
            "provenance": {
                "adapter_version": factory._current_product_adapter_version(  # noqa: SLF001
                    variant
                ),
                "compilation_receipt": receipt,
                "projected_recipe_schema_version": 3,
                "projected_recipe_sha256": f"{index + 1}" * 64,
                "dataset_id": dataset_id,
                "estimator_method_version": factory.METHOD_VERSION,
            },
            "estimation": estimation,
        }
        run_id = f"00000000-0000-0000-0000-{0x3000 + index:012x}"
        document_id = f"result_{run_id}"
        canonical = {
            "schema_version": 2,
            "document_id": document_id,
            "title": "PLS-SEM results",
            "provenance": {
                "run_id": run_id,
                "project_id": project_id,
                "model_id": model["id"],
                "model_digest": model_digest,
                "dataset_id": dataset_id,
                "dataset_fingerprint": factory.CURRENT_PRODUCT_DATASET_FINGERPRINT.removeprefix(
                    "v2:"
                ),
                "recipe_id": recipe_id,
                "recipe_digest": receipt["recipe_analytical_sha256"],
                "capability_cell": capability_cell,
                "method_version": factory.METHOD_VERSION,
                "engine_version": factory._current_product_adapter_version(  # noqa: SLF001
                    variant
                ),
                "seed": 20_260_815,
                "workers": 1,
                "started_at": f"2026-08-15T00:00:{index * 2:02}Z",
                "completed_at": f"2026-08-15T00:00:{index * 2 + 1:02}Z",
            },
            "capability_cells": [capability_cell],
            "sections": [
                {
                    "id": "run_details",
                    "title": "Run details",
                    "description": "Synthetic run details.",
                    "table_ids": [
                        "estimation_summary",
                        "point_estimate_attribution",
                        *(
                            [
                                "algorithm_convergence_receipt",
                                "algorithm_block_order",
                            ]
                            if algorithm_convergence_receipt is not None
                            else []
                        ),
                    ],
                    "chart_ids": [],
                    "capability_cells": [capability_cell],
                },
                {
                    "id": "measurement_model",
                    "title": "Measurement model",
                    "description": "Synthetic measurement model.",
                    "table_ids": ["outer_model"],
                    "chart_ids": [],
                    "capability_cells": [capability_cell],
                },
                {
                    "id": "structural_model",
                    "title": "Structural model",
                    "description": "Synthetic structural model.",
                    "table_ids": ["structural_paths", "effects", "r_squared"],
                    "chart_ids": [],
                    "capability_cells": [capability_cell],
                },
            ],
            "tables": [
                cls._synthetic_canonical_table(
                    table_id="estimation_summary",
                    columns=[
                        ("converged", "boolean"),
                        ("iterations", "number"),
                        ("used_observations", "number"),
                        ("omitted_observations", "number"),
                    ],
                    rows=[("run", [True, 4.0, 6.0, 0.0])],
                    capability_cell=capability_cell,
                ),
                cls._synthetic_canonical_table(
                    table_id="outer_model",
                    columns=[
                        ("construct", "text"),
                        ("indicator", "text"),
                        ("weight", "number"),
                        ("loading", "number"),
                    ],
                    rows=[
                        (
                            f"outer_{row_index:04}",
                            [
                                row["construct"],
                                row["indicator"],
                                row["weight"],
                                row["loading"],
                            ],
                        )
                        for row_index, row in enumerate(outer_estimates)
                    ],
                    capability_cell=capability_cell,
                ),
                cls._synthetic_canonical_table(
                    table_id="structural_paths",
                    columns=[
                        ("source", "text"),
                        ("target", "text"),
                        ("coefficient", "number"),
                    ],
                    rows=[("path_0000", ["construct:x", "construct:y", coefficient])],
                    capability_cell=capability_cell,
                ),
                cls._synthetic_canonical_table(
                    table_id="effects",
                    columns=[
                        ("source", "text"),
                        ("target", "text"),
                        ("direct", "number"),
                        ("indirect", "number"),
                        ("total", "number"),
                    ],
                    rows=[
                        (
                            "effect_0000",
                            [
                                "construct:x",
                                "construct:y",
                                coefficient,
                                0.0,
                                coefficient,
                            ],
                        )
                    ],
                    capability_cell=capability_cell,
                ),
                cls._synthetic_canonical_table(
                    table_id="r_squared",
                    columns=[("construct", "text"), ("r_squared", "number")],
                    rows=[
                        ("r_squared_0000", ["construct:y", coefficient * coefficient])
                    ],
                    capability_cell=capability_cell,
                ),
                cls._synthetic_canonical_table(
                    table_id="point_estimate_attribution",
                    columns=[
                        ("contract_version", "text"),
                        ("preprocessing", "text"),
                        ("indicator_centering", "text"),
                        ("indicator_scaling", "text"),
                        ("outer_weights", "text"),
                        ("outer_loadings", "text"),
                        ("construct_scores", "text"),
                        ("structural_paths", "text"),
                        ("effects", "text"),
                    ],
                    rows=[
                        (
                            "attribution",
                            [
                                point_estimate_attribution["contract_version"],
                                point_estimate_attribution["preprocessing"],
                                point_estimate_attribution["indicator_centering"],
                                point_estimate_attribution["indicator_scaling"],
                                point_estimate_attribution["outer_weights"],
                                point_estimate_attribution["outer_loadings"],
                                point_estimate_attribution["construct_scores"],
                                point_estimate_attribution["structural_paths"],
                                point_estimate_attribution["effects"],
                            ],
                        )
                    ],
                    capability_cell=capability_cell,
                ),
                *(
                    [
                        cls._synthetic_canonical_table(
                            table_id="algorithm_convergence_receipt",
                            columns=[
                                ("contract_version", "text"),
                                ("weighting_scheme", "text"),
                                ("maximum_iterations", "number"),
                                ("stop_criterion", "number"),
                                ("comparison", "text"),
                                ("performed_iterations", "number"),
                                ("estimated_block_updates", "number"),
                                ("termination_reason", "text"),
                                ("final_max_outer_weight_change", "number"),
                            ],
                            rows=[
                                (
                                    "convergence",
                                    [
                                        algorithm_convergence_receipt[
                                            "contract_version"
                                        ],
                                        algorithm_convergence_receipt[
                                            "weighting_scheme"
                                        ],
                                        float(
                                            algorithm_convergence_receipt[
                                                "maximum_iterations"
                                            ]
                                        ),
                                        algorithm_convergence_receipt[
                                            "stop_criterion"
                                        ],
                                        algorithm_convergence_receipt["comparison"],
                                        float(
                                            algorithm_convergence_receipt[
                                                "performed_iterations"
                                            ]
                                        ),
                                        float(
                                            algorithm_convergence_receipt[
                                                "estimated_block_updates"
                                            ]
                                        ),
                                        algorithm_convergence_receipt[
                                            "termination_reason"
                                        ],
                                        algorithm_convergence_receipt[
                                            "final_max_outer_weight_change"
                                        ],
                                    ],
                                )
                            ],
                            capability_cell=capability_cell,
                        ),
                        cls._synthetic_canonical_table(
                            table_id="algorithm_block_order",
                            columns=[
                                ("block_ordinal", "number"),
                                ("construct_id", "text"),
                                ("indicator_ordinal", "number"),
                                ("indicator_id", "text"),
                                ("update_rule", "text"),
                                ("initialization", "text"),
                            ],
                            rows=[
                                (
                                    "algorithm_block_"
                                    f"{block_index:04}_indicator_{indicator_index:04}",
                                    [
                                        float(block_index),
                                        block["construct_id"],
                                        float(indicator_index),
                                        indicator,
                                        block["update_rule"],
                                        block["initialization"],
                                    ],
                                )
                                for block_index, block in enumerate(
                                    algorithm_convergence_receipt["blocks"]
                                )
                                for indicator_index, indicator in enumerate(
                                    block["indicator_order"]
                                )
                            ],
                            capability_cell=capability_cell,
                        ),
                    ]
                    if algorithm_convergence_receipt is not None
                    else []
                ),
            ],
            "charts": [],
            "notices": [],
            "exclusions": [
                {
                    "id": "point_estimation_only",
                    "capability_cell": capability_cell,
                    "title": "Point estimation only",
                    "reason": "Synthetic point-estimation exclusion.",
                }
            ],
            "footnotes": [],
            "presentation": {
                "default_section_id": "structural_model",
                "default_table_id": "structural_paths",
                "precision": 4,
                "missing_value_label": "—",
                "chart_defaults": {},
            },
        }
        return {
            "request": request,
            "compiled": compiled,
            "execution": execution,
            "canonical": canonical,
            "receipt": receipt,
            "compilation_receipt_sha256": factory._serialized_json_sha256(  # noqa: SLF001
                cls._sorted_json_value(receipt)
            ),
        }

    @classmethod
    def _synthetic_current_product_work(cls, result_root: Path) -> dict[str, object]:
        sources = factory.source_descriptors()
        source_hash = canonical_sha256(sources)
        spec = strict_load_json(factory.SPEC_PATH)
        scenario_hash = canonical_sha256(spec["scenario_contract"])
        source_root = result_root / f"source_{source_hash}"
        binary_root = result_root / "synthetic-bin"
        binary_root.mkdir(parents=True)
        binary_path = binary_root / "quickpls_desktop_lib-synthetic.exe"
        binary_path.write_bytes(b"source-bound synthetic test executable")
        binary_sha256 = factory.sha256_file(binary_path)
        harness_root = source_root / "product_builds" / f"harness_{binary_sha256}"
        harness_dir = harness_root / "harness"
        harness_dir.mkdir(parents=True)

        capability_cell = {
            "registry_schema_version": 2,
            "capability_id": factory.CAPABILITY_ID,
            "cell_id": factory.CELL_ID,
            "capability_version": factory.METHOD_VERSION,
        }
        project_id = "00000000-0000-0000-0000-00000000a001"
        dataset_id = "00000000-0000-0000-0000-00000000d001"
        archive_publish_path = "D:/synthetic/current-product.schema6.json"
        chains = {
            variant: cls._synthetic_variant_documents(
                variant=variant,
                dataset_id=dataset_id,
                project_id=project_id,
                capability_cell=capability_cell,
            )
            for variant in factory.CURRENT_PRODUCT_VARIANTS
        }
        attachments: list[dict[str, object]] = []
        for variant in factory.CURRENT_PRODUCT_VARIANTS:
            canonical = chains[variant]["canonical"]
            assert isinstance(canonical, dict)
            attachments.append(
                {
                    "document_id": canonical["document_id"],
                    "run_id": canonical["provenance"]["run_id"],  # type: ignore[index]
                    "document_schema_version": 2,
                    "canonical_document": canonical,
                    "canonical_document_sha256": canonical_sha256(canonical),
                    "immutable": True,
                }
            )
        archive_base = {
            "created_at": "2026-08-15T00:00:00Z",
            "datasets": [
                {
                    "fingerprint": factory.CURRENT_PRODUCT_DATASET_FINGERPRINT,
                    "id": dataset_id,
                    "name": "simple_reflective.csv",
                    "schema": {
                        "case_count": 6,
                        "columns": [
                            {
                                "column_type": "numeric",
                                "label": None,
                                "missing_markers": ["", "NA", "N/A", "."],
                                "name": indicator,
                                "scale_type": "continuous",
                                "theoretical_max": None,
                                "theoretical_min": None,
                                "value_labels": {},
                            }
                            for indicator in ("x1", "x2", "y1", "y2")
                        ],
                        "kind": "raw",
                        "sample_size": None,
                        "version": 1,
                    },
                }
            ],
            "historical_recipes": [],
            "historical_results": [],
            "layouts": {},
            "models": [
                {
                    "model_id": chains[variant]["request"]["model"]["id"],  # type: ignore[index]
                    "payload": {
                        "kind": "sem_model_v4",
                        "model": chains[variant]["request"]["model"],  # type: ignore[index]
                        "scientific_sha256": chains[variant]["receipt"][  # type: ignore[index]
                            "model_scientific_sha256"
                        ],
                    },
                }
                for variant in factory.CURRENT_PRODUCT_VARIANTS
            ],
            "modified_at": "2026-08-15T00:00:00Z",
            "name": "PLS current-product work evidence",
            "project_id": project_id,
            "recipes": [
                chains[variant]["request"]["recipe"]  # type: ignore[index]
                for variant in factory.CURRENT_PRODUCT_VARIANTS
            ],
            "schema_version": 6,
            "origin": {
                "kind": "upgraded_copy",
                "lineage": {
                    "destination_archive_path": archive_publish_path,
                    "historical_results_immutable": True,
                    "source_archive_path": "D:/synthetic/unchanged-source-v5.qpls",
                    "source_archive_schema_version": 5,
                    "source_archive_sha256": "a" * 64,
                    "source_preservation": "required",
                    "source_project_id": project_id,
                    "upgraded_at": "2026-08-15T00:00:00Z",
                    "write_policy": "new_archive_only",
                },
            },
        }
        initial_archive = dict(archive_base)
        initial_document = cls._write_artifact(
            harness_dir, "archive_steps/00-initial.schema6.json", initial_archive
        )

        artifact_descriptors: dict[str, dict[str, dict[str, object]]] = {}
        for variant in factory.CURRENT_PRODUCT_VARIANTS:
            chain = chains[variant]
            artifact_descriptors[variant] = {
                "request": cls._write_artifact(
                    harness_dir, f"requests/{variant}.request.json", chain["request"]
                ),
                "compiled_artifact": cls._write_artifact(
                    harness_dir,
                    f"compiled/{variant}.compiled.json",
                    chain["compiled"],
                ),
                "execution": cls._write_artifact(
                    harness_dir,
                    f"executions/{variant}.execution.json",
                    chain["execution"],
                ),
                "canonical_document": cls._write_artifact(
                    harness_dir,
                    f"canonical/{variant}.canonical.json",
                    chain["canonical"],
                ),
            }

        variants = []
        append_source_sha256 = str(initial_document["sha256"])
        archive_step_descriptors = []
        for index, (variant, (weighting, mode)) in enumerate(
            factory.CURRENT_PRODUCT_VARIANTS.items(), start=1
        ):
            canonical = chains[variant]["canonical"]
            assert isinstance(canonical, dict)
            archive_document = {
                **archive_base,
                "canonical_result_documents": attachments[:index],
            }
            archive_after_append = cls._write_artifact(
                harness_dir,
                f"archive_steps/{index:02}-{variant}.schema6.json",
                archive_document,
            )
            archive_step_descriptors.append(archive_after_append)
            updated_sha256 = str(archive_after_append["sha256"])
            descriptors = artifact_descriptors[variant]
            variants.append(
                {
                    "variant": variant,
                    "weighting_scheme": weighting,
                    "measurement_mode": mode,
                    "recipe_v4_compiled": True,
                    "real_runner_executed": True,
                    "product_canonical_builder_executed": True,
                    "schema6_append_reopened": True,
                    "method_version": factory.METHOD_VERSION,
                    "compilation_receipt_sha256": chains[variant][
                        "compilation_receipt_sha256"
                    ],
                    "request": descriptors["request"],
                    "compiled_artifact": descriptors["compiled_artifact"],
                    "execution": descriptors["execution"],
                    "canonical_document": descriptors["canonical_document"],
                    "append_receipt": {
                        "schema_version": 6,
                        "project_id": project_id,
                        "archive_path": archive_publish_path,
                        "source_document_sha256": append_source_sha256,
                        "updated_document_sha256": updated_sha256,
                        "canonical_document_id": canonical["document_id"],
                        "run_id": canonical["provenance"]["run_id"],  # type: ignore[index]
                        "canonical_result_document_count": index,
                        "source_verified_at_commit": True,
                        "post_write_validated": True,
                        "rollback_copy_removed": True,
                    },
                    "archive_after_append": archive_after_append,
                }
            )
            append_source_sha256 = updated_sha256
        final_archive = {
            **archive_base,
            "canonical_result_documents": attachments,
        }
        archive_descriptor = cls._write_artifact(
            harness_dir, "current-product.schema6.json", final_archive
        )
        if archive_descriptor["sha256"] != archive_step_descriptors[-1]["sha256"]:
            raise AssertionError(
                "synthetic final archive differs from final append snapshot"
            )

        harness_report = {
            "schema_version": 1,
            "report_kind": "pls_algorithm_v1_current_product_harness_v1",
            "work_evidence_only": True,
            "qualification_ready": False,
            "promotion_authority": False,
            "customer_cli_invoked": False,
            "customer_registry_admission_invoked": False,
            "candidate_receipt_descriptors": [],
            "compiled_identity": {
                "source_set_sha256": source_hash,
                "scenario_set_sha256": scenario_hash,
                "executable": {
                    "path": str(binary_path.resolve()),
                    "size_bytes": binary_path.stat().st_size,
                    "sha256": binary_sha256,
                },
            },
            "capability_cell": capability_cell,
            "variants": variants,
            "archive": {
                "schema_version": 6,
                "canonical_result_document_count": 4,
                "resident_dataset_verified": True,
                "models_and_recipes_verified": True,
                "atomic_append_verified": True,
                "final_reopen_verified": True,
                "source_archive_preserved_as_absent_fixture": True,
                "initial_write": {
                    "schema_version": 6,
                    "project_id": project_id,
                    "destination_archive_path": archive_publish_path,
                    "document_sha256": initial_document["sha256"],
                    "byte_length": initial_document["size_bytes"],
                    "post_write_validated": True,
                },
                "initial_document": initial_document,
                "document": archive_descriptor,
            },
        }
        harness_report_descriptor = cls._write_artifact(
            harness_root, "harness/harness_report.json", harness_report
        )
        build_stdout = cls._write_artifact(
            harness_root, "commands/build.stdout.txt", b"{}\n"
        )
        build_stderr = cls._write_artifact(
            harness_root, "commands/build.stderr.txt", b""
        )
        run_stdout = cls._write_artifact(
            harness_root, "commands/run.stdout.txt", b"test result: ok\n"
        )
        run_stderr = cls._write_artifact(harness_root, "commands/run.stderr.txt", b"")
        newest_source_ns = max(
            (ROOT / row["path"]).stat().st_mtime_ns for row in sources
        )
        registry = strict_load_json(factory.REGISTRY_PATH)
        envelope = {
            "schema_version": 2,
            "report_kind": "pls_algorithm_v1_current_product_work_envelope_v2",
            "qualification_id": factory.QUALIFICATION_ID,
            "capability_id": factory.CAPABILITY_ID,
            "cell_id": factory.CELL_ID,
            "method_version": factory.METHOD_VERSION,
            "generated_at_utc": "2026-08-15T00:00:00Z",
            "source_set_sha256": source_hash,
            "scenario_set_sha256": scenario_hash,
            "source_artifacts": sources,
            "spec_sha256": factory.sha256_file(factory.SPEC_PATH),
            "registry": {
                "path": factory.repository_path(factory.REGISTRY_PATH),
                "raw_sha256": factory.sha256_file(factory.REGISTRY_PATH),
                "canonical_sha256": canonical_sha256(registry),
                "registry_id": registry["registry_id"],
                "registry_version": registry["registry_version"],
                "exact_cell": {
                    "capability_id": factory.CAPABILITY_ID,
                    "cell_id": factory.CELL_ID,
                    "capability_version": factory.METHOD_VERSION,
                    "coverage_state": "partial",
                    "evidence_state": "absent",
                    "surface": "labs",
                },
            },
            "harness_binary": {
                "path": factory.repository_path(binary_path),
                "sha256": binary_sha256,
                "size_bytes": binary_path.stat().st_size,
                "mtime_ns": binary_path.stat().st_mtime_ns,
                "newest_bound_source_mtime_ns": newest_source_ns,
            },
            "build_fingerprint": (
                f"quickpls-desktop-test-sha256:{binary_sha256};"
                f"source-set-sha256:{source_hash}"
            ),
            "passed": True,
            "work_evidence_only": True,
            "qualification_ready": False,
            "promotion_authority": False,
            "candidate_receipt_descriptors": [],
            "customer_cli_invoked": False,
            "customer_registry_admission_invoked": False,
            "commands": [
                {
                    "label": "build_internal_desktop_harness",
                    "command": [
                        "cargo",
                        "test",
                        "-p",
                        "quickpls-desktop",
                        "--lib",
                        "--no-run",
                        "--message-format=json",
                    ],
                    "returncode": 0,
                    "started_at_utc": "2026-08-15T00:00:00Z",
                    "finished_at_utc": "2026-08-15T00:00:01Z",
                    "duration_seconds": 1.0,
                    "stdout": build_stdout,
                    "stderr": build_stderr,
                },
                {
                    "label": "run_internal_desktop_harness",
                    "command": [
                        str(binary_path.resolve()),
                        "--ignored",
                        "--exact",
                        factory.INTERNAL_HARNESS_TEST,
                        "--nocapture",
                        "--test-threads=1",
                    ],
                    "returncode": 0,
                    "started_at_utc": "2026-08-15T00:00:01Z",
                    "finished_at_utc": "2026-08-15T00:00:02Z",
                    "duration_seconds": 1.0,
                    "stdout": run_stdout,
                    "stderr": run_stderr,
                },
            ],
            "comparisons": [
                {
                    "variant": variant,
                    "passed": True,
                    "exact_key_membership": True,
                    "compared_quantity_count": 9,
                    "max_abs_difference": index * 1.0e-16,
                    "tolerance": 1.0e-6,
                    "missing_from_product": [],
                    "unexpected_from_product": [],
                    "construct_identity_map": {
                        "x": "construct:x",
                        "y": "construct:y",
                    },
                }
                for index, variant in enumerate(
                    factory.CURRENT_PRODUCT_VARIANTS, start=1
                )
            ],
            "archive_smoke": {
                "passed": True,
                "schema_version": 6,
                "scientific_result_appended_by_real_runner": True,
                "real_product_canonical_projection": True,
                "atomic_append_and_reopen_verified": True,
                "receipt_eligible": False,
            },
            "harness_report": harness_report_descriptor,
            "remaining_blockers": list(factory.CURRENT_PRODUCT_WORK_BLOCKERS),
        }
        envelope_path = harness_root / "current_product_work_envelope.json"
        cls._write_json(envelope_path, envelope)
        return {
            "source_hash": source_hash,
            "scenario_hash": scenario_hash,
            "result_root": result_root,
            "allowed_binary_root": binary_root,
            "source_root": source_root,
            "harness_root": harness_root,
            "envelope_path": envelope_path,
            "binary_path": binary_path,
            "canonical_path": harness_dir / "canonical/path_mode_a.canonical.json",
            "request_path": harness_dir / "requests/path_mode_a.request.json",
            "compiled_path": harness_dir / "compiled/path_mode_a.compiled.json",
            "execution_path": harness_dir / "executions/path_mode_a.execution.json",
            "initial_archive_path": harness_dir
            / "archive_steps/00-initial.schema6.json",
            "first_archive_step_path": harness_dir
            / "archive_steps/01-path_mode_a.schema6.json",
        }

    @staticmethod
    def _descriptor_for(root: Path, path: Path) -> dict[str, object]:
        return {
            "path": path.relative_to(root).as_posix(),
            "size_bytes": path.stat().st_size,
            "sha256": factory.sha256_file(path),
        }

    @classmethod
    def _publish_tampered_harness_report(
        cls, fixture: dict[str, object], report: dict[str, object]
    ) -> None:
        harness_root = Path(fixture["harness_root"])
        report_path = harness_root / "harness/harness_report.json"
        cls._write_json(report_path, report)
        envelope_path = Path(fixture["envelope_path"])
        envelope = strict_load_json(envelope_path)
        envelope["harness_report"] = cls._descriptor_for(harness_root, report_path)
        cls._write_json(envelope_path, envelope)

    @classmethod
    def _publish_tampered_variant_artifact(
        cls,
        fixture: dict[str, object],
        *,
        variant: str,
        artifact_name: str,
        artifact_path: Path,
    ) -> None:
        harness_root = Path(fixture["harness_root"])
        harness_dir = harness_root / "harness"
        report_path = harness_dir / "harness_report.json"
        report = strict_load_json(report_path)
        row = next(row for row in report["variants"] if row["variant"] == variant)
        row[artifact_name] = cls._descriptor_for(harness_dir, artifact_path)
        cls._publish_tampered_harness_report(fixture, report)

    def test_current_spec_is_valid_fail_closed_and_registry_linked(self) -> None:
        spec = factory.build_spec()
        validation = validate_spec_document(
            spec,
            repository_root=ROOT,
            registry_document=strict_load_json(factory.REGISTRY_PATH),
            require_registry=True,
        )
        self.assertTrue(validation["passed"], validation["errors"])
        self.assertTrue(validation["registry_verified"])
        self.assertFalse(validation["qualification_ready"])
        self.assertEqual(spec["migration"]["status"], "compatibility_only")
        self.assertEqual(spec["evidence_contract"]["receipts"], [])
        self.assertEqual(
            tuple(spec["evidence_contract"]["required_roles"]),
            factory.EXPECTED_REQUIRED_ROLES,
        )

    def test_future_full_parity_backlog_is_distinct_and_non_promotional(self) -> None:
        unresolved = set(factory.build_spec()["migration"]["unresolved_items"])
        self.assertTrue(set(factory.FULL_PARITY_COVERAGE_BACKLOG).issubset(unresolved))
        self.assertTrue(set(factory.FULL_PARITY_EVIDENCE_BACKLOG).issubset(unresolved))
        self.assertTrue(
            all(
                value.startswith("future_full_parity.coverage.")
                for value in factory.FULL_PARITY_COVERAGE_BACKLOG
            )
        )
        self.assertTrue(
            all(
                value.startswith("future_full_parity.evidence.")
                for value in factory.FULL_PARITY_EVIDENCE_BACKLOG
            )
        )
        coverage = "\n".join(factory.FULL_PARITY_COVERAGE_BACKLOG)
        self.assertNotIn("initial_outer_weights_standard_and_individual", coverage)
        self.assertNotIn("not_implemented_or_qualified", coverage)
        self.assertIn("bounded_unit_variance_execution", coverage)
        self.assertIn(
            "future_full_parity.evidence.standard_individual_initial_outer_weights_matrix",
            factory.FULL_PARITY_EVIDENCE_BACKLOG,
        )
        self.assertIn(
            "future_full_parity.evidence.unit_custom_mixed_fixed_only_score_execution_matrix",
            factory.FULL_PARITY_EVIDENCE_BACKLOG,
        )

    def test_archive_scale_and_official_defaults_match_current_contract(self) -> None:
        spec = factory.build_spec()
        archive = spec["operational_contract"]["archive"]
        self.assertEqual(archive["current_schema_version"], 6)
        self.assertEqual(archive["readable_schema_versions"], [1, 2, 3, 4, 5, 6])
        self.assertEqual(archive["writable_schema_versions"], [6])
        maximum = next(
            row
            for row in spec["scenario_contract"]["complexity_profiles"]
            if row["id"] == "maximum_axis"
        )["workload"]
        self.assertEqual(maximum["rows"], 100_000)
        self.assertEqual(maximum["indicators"], 300)
        self.assertEqual(maximum["constructs"], 100)
        method = (ROOT / "docs/methods/PLS_PM_V1.md").read_text(encoding="utf-8")
        self.assertIn("3,000", method)
        self.assertIn("`1e-7`", method)

    def test_checked_in_spec_equals_deterministic_builder(self) -> None:
        self.assertEqual(strict_load_json(factory.SPEC_PATH), factory.build_spec())

    def test_snapshot_publication_preserves_frozen_spec_bytes_and_mtime(self) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            root = Path(directory)
            spec_path = root / "pls_algorithm_v1.qualification.json"
            result_root = root / "results"
            self._write_json(spec_path, factory.build_spec())
            before_bytes = spec_path.read_bytes()
            before_mtime_ns = spec_path.stat().st_mtime_ns
            with (
                mock.patch.object(factory, "SPEC_PATH", spec_path),
                mock.patch.object(factory, "RESULT_ROOT", result_root),
                mock.patch.object(
                    factory,
                    "validate_current_product_work_envelope",
                    side_effect=RuntimeError("stop after frozen-spec preflight"),
                ),
                self.assertRaisesRegex(RuntimeError, "frozen-spec preflight"),
            ):
                factory.write_factory_snapshot()
            self.assertEqual(spec_path.read_bytes(), before_bytes)
            self.assertEqual(spec_path.stat().st_mtime_ns, before_mtime_ns)
            self.assertFalse(result_root.exists())

    def test_snapshot_publication_rejects_spec_mismatch_before_partial_output(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            root = Path(directory)
            spec_path = root / "pls_algorithm_v1.qualification.json"
            result_root = root / "results"
            stale = copy.deepcopy(factory.build_spec())
            stale["migration"]["status"] = "stale-test-value"
            self._write_json(spec_path, stale)
            with (
                mock.patch.object(factory, "SPEC_PATH", spec_path),
                mock.patch.object(factory, "RESULT_ROOT", result_root),
                mock.patch.object(
                    factory, "validate_current_product_work_envelope"
                ) as validator,
                self.assertRaisesRegex(
                    ValueError, "--refresh-spec before source freeze"
                ),
            ):
                factory.write_factory_snapshot()
            validator.assert_not_called()
            self.assertFalse(result_root.exists())

            refreshed = factory.refresh_qualification_spec(path=spec_path)
            self.assertTrue(refreshed["refreshed"])
            self.assertEqual(strict_load_json(spec_path), factory.build_spec())
            before_bytes = spec_path.read_bytes()
            before_mtime_ns = spec_path.stat().st_mtime_ns
            unchanged = factory.refresh_qualification_spec(path=spec_path)
            self.assertFalse(unchanged["refreshed"])
            self.assertEqual(spec_path.read_bytes(), before_bytes)
            self.assertEqual(spec_path.stat().st_mtime_ns, before_mtime_ns)

    def test_factory_never_sources_historical_evidence_reports(self) -> None:
        self.assertFalse(
            any(
                path.startswith("validation/results/method_factory/pls_algorithm_v1/")
                for path in factory.SOURCE_PATHS
            )
        )
        self.assertNotIn(
            "validation/qualification_v2/fixtures/evidence",
            "\n".join(factory.SOURCE_PATHS),
        )

    def test_current_build_generator_rejects_source_hash_mismatch(self) -> None:
        with self.assertRaisesRegex(ValueError, "source-set SHA mismatch"):
            build_receipts._assert_exact_identity(  # noqa: SLF001
                source_set_sha256="0" * 64,
                scenario_set_sha256="0" * 64,
            )

    def test_current_build_generator_rejects_stale_harness_binary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fake_harness = Path(directory) / "quickpls_desktop_lib-test.exe"
            fake_harness.write_bytes(b"not executable")
            os.utime(fake_harness, (1, 1))
            bound_source = factory.source_descriptors()[0]
            with self.assertRaisesRegex(ValueError, "predates a bound source"):
                build_receipts._bind_harness_binary(  # noqa: SLF001
                    fake_harness,
                    {"source_artifacts": [bound_source]},
                )

    def test_current_product_matrix_and_commands_are_exact_and_cli_independent(
        self,
    ) -> None:
        self.assertEqual(
            build_receipts.VARIANTS,
            {
                "path_mode_a": ("path", "mode_a"),
                "path_mode_b": ("path", "mode_b"),
                "factor_mode_a": ("factor", "mode_a"),
                "pca_mode_a": ("pca", "mode_a"),
            },
        )
        commands = [
            *build_receipts._cargo_build_command("cargo"),  # noqa: SLF001
            *build_receipts._harness_command(  # noqa: SLF001
                Path("target/debug/deps/quickpls_desktop_lib-test.exe")
            ),
        ]
        flattened = " ".join(map(str, commands)).lower()
        self.assertNotIn("qpls run", flattened)
        self.assertNotIn("allow-experimental", flattened)
        self.assertNotIn("qpls-cli", flattened)

        source_hash = "1" * 64
        scenario_hash = "2" * 64
        contract = factory._current_build_execution_contract(  # noqa: SLF001
            source_hash, scenario_hash
        )
        self.assertEqual(
            contract["central_commands"],
            [
                [
                    "python",
                    factory.INTERNAL_HARNESS_ORCHESTRATOR,
                    "--source-set-sha256",
                    source_hash,
                    "--scenario-set-sha256",
                    scenario_hash,
                    "--write-new-snapshot",
                ]
            ],
        )
        self.assertFalse(contract["customer_cli_execution_allowed"])
        self.assertFalse(contract["candidate_receipt_output_allowed"])
        serialized = str(contract).lower()
        self.assertNotIn("qpls-cli", serialized)
        self.assertNotIn("qpls.exe", serialized)
        self.assertNotIn("--cli", serialized)
        self.assertNotIn("allow-experimental", serialized)

    def test_factory_describes_only_the_source_bound_internal_product_harness(
        self,
    ) -> None:
        harness = factory.describe_internal_product_harness()
        self.assertEqual(harness["surface"], "internal_labs_ignored_lib_test")
        self.assertEqual(harness["package"], "quickpls-desktop")
        self.assertEqual(harness["target"], "quickpls_desktop_lib")
        self.assertEqual(harness["test"], factory.INTERNAL_HARNESS_TEST)
        self.assertFalse(harness["product_execution_performed_by_factory"])
        self.assertFalse(harness["customer_cli_invoked"])
        self.assertFalse(harness["customer_registry_admission_invoked"])
        self.assertFalse(harness["candidate_receipt_output_allowed"])
        self.assertFalse(harness["qualification_ready"])
        self.assertEqual(
            [row["path"] for row in harness["product_source_artifacts"]],
            sorted(factory.PRODUCT_BUILD_SOURCE_PATHS),
        )
        self.assertFalse(
            any("qpls-cli" in path for path in factory.PRODUCT_BUILD_SOURCE_PATHS)
        )

    def test_current_product_work_discovery_requires_exactly_one_envelope(self) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            result_root = Path(directory)
            source_hash = canonical_sha256(factory.source_descriptors())
            scenario_hash = canonical_sha256(factory.build_spec()["scenario_contract"])
            with self.assertRaisesRegex(ValueError, "exactly one.*found 0"):
                factory.validate_current_product_work_envelope(
                    source_hash=source_hash,
                    scenario_hash=scenario_hash,
                    result_root=result_root,
                    allowed_binary_root=result_root,
                )

            fixture = self._synthetic_current_product_work(result_root)
            summary = factory.validate_current_product_work_envelope(
                source_hash=str(fixture["source_hash"]),
                scenario_hash=str(fixture["scenario_hash"]),
                result_root=result_root,
                allowed_binary_root=Path(fixture["allowed_binary_root"]),
            )
            self.assertTrue(summary["kernel_execution_work_evidence"])
            self.assertTrue(summary["archive_persistence_work_evidence"])
            self.assertFalse(summary["receipt_eligible"])
            self.assertFalse(summary["qualification_ready"])

            duplicate = Path(fixture["harness_root"]).with_name("harness_duplicate")
            shutil.copytree(Path(fixture["harness_root"]), duplicate)
            with self.assertRaisesRegex(ValueError, "exactly one.*found 2"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=result_root,
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_fixture_has_exact_mixed_adapter_generations(self) -> None:
        capability_cell = {
            "registry_schema_version": 2,
            "capability_id": factory.CAPABILITY_ID,
            "cell_id": factory.CELL_ID,
            "capability_version": factory.METHOD_VERSION,
        }
        for variant in factory.CURRENT_PRODUCT_VARIANTS:
            with self.subTest(variant=variant):
                documents = self._synthetic_variant_documents(
                    variant=variant,
                    dataset_id="00000000-0000-0000-0000-00000000d001",
                    project_id="00000000-0000-0000-0000-00000000a001",
                    capability_cell=capability_cell,
                )
                execution = documents["execution"]
                canonical = documents["canonical"]
                self.assertEqual(
                    execution["provenance"]["adapter_version"],  # type: ignore[index]
                    factory.CURRENT_PRODUCT_ADAPTER_VERSIONS[variant],
                )
                self.assertEqual(
                    canonical["provenance"]["engine_version"],  # type: ignore[index]
                    factory.CURRENT_PRODUCT_ADAPTER_VERSIONS[variant],
                )
                estimation = execution["estimation"]  # type: ignore[index]
                table_ids = [table["id"] for table in canonical["tables"]]  # type: ignore[index]
                self.assertIn("point_estimate_attribution", estimation)
                self.assertIn("point_estimate_attribution", table_ids)
                if variant == "pca_mode_a":
                    self.assertNotIn("algorithm_convergence_receipt", estimation)
                    self.assertNotIn("algorithm_convergence_receipt", table_ids)
                    self.assertNotIn("algorithm_block_order", table_ids)
                    self.assertEqual(len(table_ids), 6)
                else:
                    self.assertIn("algorithm_convergence_receipt", estimation)
                    self.assertIn("algorithm_convergence_receipt", table_ids)
                    self.assertIn("algorithm_block_order", table_ids)
                    self.assertEqual(len(table_ids), 8)

    def test_current_product_work_rejects_attribution_and_adapter_tampering(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            execution_path = Path(fixture["execution_path"])
            execution = strict_load_json(execution_path)
            del execution["estimation"]["point_estimate_attribution"]
            self._write_json(execution_path, execution)
            self._publish_tampered_variant_artifact(
                fixture,
                variant="path_mode_a",
                artifact_name="execution",
                artifact_path=execution_path,
            )
            with self.assertRaisesRegex(
                ValueError, "missing=.*point_estimate_attribution"
            ):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            execution_path = Path(fixture["execution_path"])
            execution = strict_load_json(execution_path)
            execution["provenance"][
                "adapter_version"
            ] = factory.CURRENT_PRODUCT_LEGACY_ADAPTER_VERSION
            self._write_json(execution_path, execution)
            self._publish_tampered_variant_artifact(
                fixture,
                variant="path_mode_a",
                artifact_name="execution",
                artifact_path=execution_path,
            )
            with self.assertRaisesRegex(ValueError, "execution provenance is invalid"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_work_rejects_convergence_and_section_tampering(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            execution_path = Path(fixture["execution_path"])
            execution = strict_load_json(execution_path)
            execution["estimation"]["algorithm_convergence_receipt"][
                "estimated_block_updates"
            ] += 1
            self._write_json(execution_path, execution)
            self._publish_tampered_variant_artifact(
                fixture,
                variant="path_mode_a",
                artifact_name="execution",
                artifact_path=execution_path,
            )
            with self.assertRaisesRegex(
                ValueError, "algorithm convergence receipt is invalid"
            ):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            harness_dir = Path(fixture["harness_root"]) / "harness"
            path_execution = strict_load_json(
                harness_dir / "executions/path_mode_a.execution.json"
            )
            pca_execution_path = harness_dir / "executions/pca_mode_a.execution.json"
            pca_execution = strict_load_json(pca_execution_path)
            pca_execution["estimation"]["algorithm_convergence_receipt"] = copy.deepcopy(
                path_execution["estimation"]["algorithm_convergence_receipt"]
            )
            self._write_json(pca_execution_path, pca_execution)
            self._publish_tampered_variant_artifact(
                fixture,
                variant="pca_mode_a",
                artifact_name="execution",
                artifact_path=pca_execution_path,
            )
            with self.assertRaisesRegex(
                ValueError, "unknown=.*algorithm_convergence_receipt"
            ):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            canonical_path = Path(fixture["canonical_path"])
            canonical = strict_load_json(canonical_path)
            run_details = next(
                section
                for section in canonical["sections"]
                if section["id"] == "run_details"
            )
            run_details["table_ids"].remove("point_estimate_attribution")
            self._write_json(canonical_path, canonical)
            self._publish_tampered_variant_artifact(
                fixture,
                variant="path_mode_a",
                artifact_name="canonical_document",
                artifact_path=canonical_path,
            )
            with self.assertRaisesRegex(ValueError, "section identity is invalid"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_work_rejects_canonical_receipt_cross_binding_tamper(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            canonical_path = Path(fixture["canonical_path"])
            canonical = strict_load_json(canonical_path)
            convergence = next(
                table
                for table in canonical["tables"]
                if table["id"] == "algorithm_convergence_receipt"
            )
            convergence["rows"][0]["cells"][5]["value"] += 1.0
            self._write_json(canonical_path, canonical)
            self._publish_tampered_variant_artifact(
                fixture,
                variant="path_mode_a",
                artifact_name="canonical_document",
                artifact_path=canonical_path,
            )
            with self.assertRaisesRegex(
                ValueError, "canonical algorithm convergence differs from execution"
            ):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_work_rejects_binary_and_canonical_tampering(self) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            Path(fixture["binary_path"]).write_bytes(b"substituted executable")
            with self.assertRaisesRegex(ValueError, "binary descriptor is stale"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            with Path(fixture["canonical_path"]).open("ab") as handle:
                handle.write(b" ")
            with self.assertRaisesRegex(
                ValueError, "canonical_document artifact.*mismatch"
            ):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_work_rejects_schema6_and_promotional_tampering(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            envelope_path = Path(fixture["envelope_path"])
            envelope = strict_load_json(envelope_path)
            envelope["archive_smoke"]["atomic_append_and_reopen_verified"] = False
            self._write_json(envelope_path, envelope)
            with self.assertRaisesRegex(ValueError, "schema-6 envelope flags"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            envelope_path = Path(fixture["envelope_path"])
            envelope = strict_load_json(envelope_path)
            envelope["remaining_blockers"] = []
            self._write_json(envelope_path, envelope)
            with self.assertRaisesRegex(ValueError, "timestamp/blockers are stale"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            envelope_path = Path(fixture["envelope_path"])
            envelope = strict_load_json(envelope_path)
            envelope["qualification_ready"] = True
            self._write_json(envelope_path, envelope)
            with self.assertRaisesRegex(ValueError, "stale or promotional"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_work_rejects_missing_unknown_and_duplicate_fields(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            report_path = Path(fixture["harness_root"]) / "harness/harness_report.json"
            report = strict_load_json(report_path)
            del report["variants"][0]["archive_after_append"]
            self._publish_tampered_harness_report(fixture, report)
            with self.assertRaisesRegex(ValueError, "missing=.*archive_after_append"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            harness_dir = Path(fixture["harness_root"]) / "harness"
            request_path = Path(fixture["request_path"])
            request = strict_load_json(request_path)
            request["unknownExecutionSurface"] = "standard"
            self._write_json(request_path, request)
            report_path = harness_dir / "harness_report.json"
            report = strict_load_json(report_path)
            report["variants"][0]["request"] = self._descriptor_for(
                harness_dir, request_path
            )
            self._publish_tampered_harness_report(fixture, report)
            with self.assertRaisesRegex(
                ValueError, "unknown=.*unknownExecutionSurface"
            ):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            report_path = Path(fixture["harness_root"]) / "harness/harness_report.json"
            report = strict_load_json(report_path)
            report["unexpected_promotion_hint"] = False
            self._publish_tampered_harness_report(fixture, report)
            with self.assertRaisesRegex(
                ValueError, "unknown=.*unexpected_promotion_hint"
            ):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            report_path = Path(fixture["harness_root"]) / "harness/harness_report.json"
            report = strict_load_json(report_path)
            report["variants"].append(copy.deepcopy(report["variants"][0]))
            self._publish_tampered_harness_report(fixture, report)
            with self.assertRaisesRegex(ValueError, "variant set is not exact"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_work_rejects_cross_variant_substitution(self) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            report_path = Path(fixture["harness_root"]) / "harness/harness_report.json"
            report = strict_load_json(report_path)
            report["variants"][0]["request"], report["variants"][2]["request"] = (
                report["variants"][2]["request"],
                report["variants"][0]["request"],
            )
            self._publish_tampered_harness_report(fixture, report)
            with self.assertRaisesRegex(ValueError, "SemModelV4 is not the exact"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_work_rejects_coordinated_numerical_tamper(self) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            harness_dir = Path(fixture["harness_root"]) / "harness"
            execution_path = Path(fixture["execution_path"])
            execution = strict_load_json(execution_path)
            execution["estimation"]["outer_estimates"][0]["weight"] += 0.25
            self._write_json(execution_path, execution)
            canonical_path = Path(fixture["canonical_path"])
            canonical = strict_load_json(canonical_path)
            outer_table = next(
                table for table in canonical["tables"] if table["id"] == "outer_model"
            )
            outer_table["rows"][0]["cells"][2]["value"] += 0.25
            self._write_json(canonical_path, canonical)
            report_path = harness_dir / "harness_report.json"
            report = strict_load_json(report_path)
            report["variants"][0]["execution"] = self._descriptor_for(
                harness_dir, execution_path
            )
            report["variants"][0]["canonical_document"] = self._descriptor_for(
                harness_dir, canonical_path
            )
            self._publish_tampered_harness_report(fixture, report)
            with self.assertRaisesRegex(ValueError, "stable product/oracle quantities"):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_current_product_work_rejects_intermediate_archive_byte_tamper(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
            fixture = self._synthetic_current_product_work(Path(directory))
            harness_dir = Path(fixture["harness_root"]) / "harness"
            step_path = Path(fixture["first_archive_step_path"])
            with step_path.open("ab") as handle:
                handle.write(b"\n")
            report_path = harness_dir / "harness_report.json"
            report = strict_load_json(report_path)
            report["variants"][0]["archive_after_append"] = self._descriptor_for(
                harness_dir, step_path
            )
            self._publish_tampered_harness_report(fixture, report)
            with self.assertRaisesRegex(
                ValueError, "canonical/append contract is invalid"
            ):
                factory.validate_current_product_work_envelope(
                    source_hash=str(fixture["source_hash"]),
                    scenario_hash=str(fixture["scenario_hash"]),
                    result_root=Path(directory),
                    allowed_binary_root=Path(fixture["allowed_binary_root"]),
                )

    def test_initial_archive_requires_serde_omission_for_empty_attachments(
        self,
    ) -> None:
        for forbidden_value in ([], [{}]):
            with self.subTest(forbidden_value=forbidden_value):
                with tempfile.TemporaryDirectory(dir=factory.RESULT_ROOT) as directory:
                    fixture = self._synthetic_current_product_work(Path(directory))
                    harness_dir = Path(fixture["harness_root"]) / "harness"
                    initial_path = Path(fixture["initial_archive_path"])
                    initial = strict_load_json(initial_path)
                    self.assertNotIn("canonical_result_documents", initial)
                    initial["canonical_result_documents"] = forbidden_value
                    self._write_json(initial_path, initial)
                    report_path = harness_dir / "harness_report.json"
                    report = strict_load_json(report_path)
                    descriptor = self._descriptor_for(harness_dir, initial_path)
                    report["archive"]["initial_document"] = descriptor
                    report["archive"]["initial_write"]["document_sha256"] = descriptor[
                        "sha256"
                    ]
                    report["archive"]["initial_write"]["byte_length"] = descriptor[
                        "size_bytes"
                    ]
                    report["variants"][0]["append_receipt"][
                        "source_document_sha256"
                    ] = descriptor["sha256"]
                    self._publish_tampered_harness_report(fixture, report)
                    with self.assertRaisesRegex(
                        ValueError, "unknown=.*canonical_result_documents"
                    ):
                        factory.validate_current_product_work_envelope(
                            source_hash=str(fixture["source_hash"]),
                            scenario_hash=str(fixture["scenario_hash"]),
                            result_root=Path(directory),
                            allowed_binary_root=Path(fixture["allowed_binary_root"]),
                        )

    def test_checked_in_snapshot_is_current_and_non_promotional(self) -> None:
        verification = factory.verify_checked_in_snapshot()
        self.assertTrue(verification["passed"], verification["errors"])
        self.assertEqual(verification["candidate_receipt_roles"], ["method_contract"])
        self.assertEqual(
            verification["work_evidence_roles"],
            [
                "kernel_execution",
                "oracle_independence",
                "adversarial_boundaries",
                "archive_persistence",
            ],
        )
        self.assertEqual(
            verification["supportable_evidence_tier"],
            "source_contract_candidate_only",
        )
        self.assertFalse(verification["qualification_ready"])
        self.assertFalse(verification["promotion_allowed"])

    def test_receipt_attachment_or_state_claim_fails_verification_contract(
        self,
    ) -> None:
        spec = factory.build_spec()
        mutated = copy.deepcopy(spec)
        mutated["migration"]["status"] = "completed"
        validation = validate_spec_document(
            mutated,
            repository_root=ROOT,
            registry_document=strict_load_json(factory.REGISTRY_PATH),
            require_registry=True,
        )
        self.assertFalse(validation["qualification_ready"])
        self.assertTrue(validation["errors"])


if __name__ == "__main__":
    unittest.main()
