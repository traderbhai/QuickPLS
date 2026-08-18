import copy
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import established_method_factory as factory_cli
import established_method_factory_common as factory


class EstablishedMethodFactoryTests(unittest.TestCase):
    def test_all_configs_bind_the_exact_manifest_identity(self):
        for method, config in factory.METHODS.items():
            with self.subTest(method=method):
                document = factory._manifest(method)
                self.assertEqual(document["feature"]["id"], config["feature_id"])
                self.assertEqual(document["feature"]["method_version"], config["method_version"])

    def test_native_stage_sources_cover_both_roles(self):
        for method in factory.METHODS:
            with self.subTest(method=method):
                sources = factory.required_sources(method, ["frontend_report", "export_report"])
                self.assertIn(factory.COMMON_SOURCE, sources)
                self.assertTrue(any(path.endswith("nativeExportTables.test.ts") for path in sources))

    def test_independent_reference_sources_bind_the_cli_policy(self):
        for method in factory.METHODS:
            with self.subTest(method=method):
                sources = factory.required_sources(method, ["independent_reference"])
                self.assertIn("crates/qpls-cli/src/main.rs", sources)

    def test_factory_rebuild_does_not_require_a_predeclared_release_state(self):
        for method in factory.METHODS:
            with self.subTest(method=method):
                qualification = factory._manifest(method)["qualification"]
                self.assertTrue(factory.method_contract_ready_for_factory(method))
                self.assertEqual(qualification["target_state"], "release_qualified")
                if qualification["declared_state"] != "release_qualified":
                    self.assertEqual(
                        qualification["evidence"]["release_qualified"],
                        [],
                    )

    def test_reference_identity_lookup_fails_closed(self):
        value = {"checks": {"nca": {"method_version": "nca_v2"}}}
        self.assertEqual(factory._pointer(value, ["checks", "nca", "method_version"]), "nca_v2")
        mutated = copy.deepcopy(value)
        del mutated["checks"]["nca"]["method_version"]
        with self.assertRaises(KeyError):
            factory._pointer(mutated, ["checks", "nca", "method_version"])

    def test_cli_without_method_arguments_runs_the_complete_factory(self):
        with (
            mock.patch.object(sys, "argv", ["established_method_factory.py"]),
            mock.patch.object(factory_cli, "qualify_all", return_value=[]) as qualify,
        ):
            self.assertEqual(factory_cli.main(), 0)
        qualify.assert_called_once_with(None)


if __name__ == "__main__":
    unittest.main()
