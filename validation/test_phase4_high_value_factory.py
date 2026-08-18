#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

from phase4_high_value_factory import (  # noqa: E402
    CONFIGS,
    DuplicateKeyError,
    _json_close,
    _mga_method_config,
    _prediction_method_config,
    strict_load,
    upgrade_recipe_v3,
)


class Phase4HighValueFactoryTests(unittest.TestCase):
    def test_micom_mga_factory_targets_v3_without_relabeling_v2(self) -> None:
        self.assertIn("micom_permutation_mga_v3", CONFIGS)
        self.assertNotIn("micom_permutation_mga_v2", CONFIGS)
        config = CONFIGS["micom_permutation_mga_v3"]
        self.assertEqual(config["method_version"], "pls_mga_permutation_v3")
        self.assertEqual(config["reference_module"], "micom_mga_v3_reference")

    def test_strict_json_rejects_duplicate_keys_and_nonfinite_values(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            duplicate = Path(folder) / "duplicate.json"
            duplicate.write_text('{"passed":true,"passed":false}\n', encoding="utf-8")
            with self.assertRaises(DuplicateKeyError):
                strict_load(duplicate)
            nonfinite = Path(folder) / "nonfinite.json"
            nonfinite.write_text('{"value":NaN}\n', encoding="utf-8")
            with self.assertRaises(ValueError):
                strict_load(nonfinite)

    def test_schema_v3_upgrade_is_non_mutating_and_explicit(self) -> None:
        source = {
            "schema_version": 2,
            "model": {"constructs": [], "paths": []},
            "settings": {"method": "predict"},
            "metadata": {"group_methods": "legacy"},
        }
        upgraded = upgrade_recipe_v3(source, _prediction_method_config(source))
        self.assertEqual(source["schema_version"], 2)
        self.assertEqual(upgraded["schema_version"], 3)
        self.assertEqual(upgraded["method_config"], {"kind": "predict"})
        self.assertEqual(upgraded["model"]["controls"], [])
        self.assertEqual(upgraded["settings"]["case_weight_column"], None)
        self.assertNotIn("group_methods", upgraded["metadata"])

    def test_mga_method_config_binds_the_exact_coupled_plan(self) -> None:
        recipe = {
            "metadata": {
                "mga_group_column": "segment",
                "mga_group_a": "A",
                "mga_group_b": "B",
                "group_permutation_samples": "5000",
                "micom_configural_confirmed": "true",
            }
        }
        self.assertEqual(
            _mga_method_config(recipe),
            {
                "kind": "mga",
                "group_column": "segment",
                "group_a": "A",
                "group_b": "B",
                "methods": ["micom", "mga_permutation"],
                "permutation_samples": 5000,
                "configural_invariance_confirmed": True,
            },
        )

    def test_recursive_numerical_comparison_is_order_and_tolerance_strict(self) -> None:
        self.assertTrue(_json_close({"a": [1.0, 2]}, {"a": [1.0 + 1e-11, 2]}))
        self.assertFalse(_json_close({"a": [1.0, 2]}, {"a": [1.0, 3]}))
        self.assertFalse(_json_close({"a": [1, 2]}, {"a": [2, 1]}))


if __name__ == "__main__":
    unittest.main()
