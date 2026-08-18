#!/usr/bin/env python3
"""Focused, Cargo-free acceptance tests for established-method contract v1."""

from __future__ import annotations

import importlib.util
import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


sys.dont_write_bytecode = True
REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
GENERATOR_PATH = REPOSITORY_ROOT / "validation" / "established_method_contract_codegen_v1.py"
SPEC = importlib.util.spec_from_file_location("established_method_contract_codegen_v1", GENERATOR_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {GENERATOR_PATH}")
codegen = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = codegen
SPEC.loader.exec_module(codegen)


class EstablishedMethodContractCodegenV1Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.live_model = codegen.load_contract_model(REPOSITORY_ROOT)
        cls.fixture_inputs = tuple(cls.live_model["input_paths"])

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="qpls-method-contract-v1-")
        self.root = Path(self.temporary.name)
        for relative in self.fixture_inputs:
            source = REPOSITORY_ROOT.joinpath(*Path(relative).parts)
            target = self.root.joinpath(*Path(relative).parts)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    @property
    def contract_path(self) -> Path:
        return self.root / codegen.CONTRACT_RELATIVE

    @property
    def registry_path(self) -> Path:
        return self.root / codegen.REGISTRY_RELATIVE

    def read_contract(self) -> dict[str, object]:
        return json.loads(self.contract_path.read_text(encoding="utf-8"))

    def write_contract(self, contract: dict[str, object]) -> None:
        self.contract_path.write_text(
            json.dumps(contract, allow_nan=False, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

    def all_files(self) -> set[str]:
        return {
            path.relative_to(self.root).as_posix()
            for path in self.root.rglob("*")
            if path.is_file()
        }

    def assert_mutation_rejected(
        self,
        relative: str,
        needle: str,
        replacement: str,
        message: str,
    ) -> None:
        path = self.root / relative
        original = path.read_text(encoding="utf-8")
        self.assertIn(needle, original)
        self.assertNotEqual(needle, replacement)
        path.write_text(original.replace(needle, replacement, 1), encoding="utf-8")
        try:
            with self.assertRaisesRegex(codegen.ContractError, message):
                codegen.load_contract_model(self.root)
        finally:
            path.write_text(original, encoding="utf-8")

    def test_01_strict_schema_and_contract_reject_duplicate_unknown_invalid_nonfinite_and_paths(self) -> None:
        schema = codegen.strict_json_file(self.root / codegen.SCHEMA_RELATIVE)
        self.assertEqual(codegen._validate_schema(schema)["$id"], codegen.EXPECTED_SCHEMA_ID)
        original = self.contract_path.read_bytes()

        def duplicate_key() -> bytes:
            return original.replace(
                b'  "schema_version": 1,',
                b'  "schema_version": 1,\n  "schema_version": 1,',
                1,
            )

        def unknown_key() -> bytes:
            contract = json.loads(original)
            contract["unexpected"] = True
            return (json.dumps(contract, indent=2) + "\n").encode("utf-8")

        def invalid_identifier() -> bytes:
            contract = json.loads(original)
            contract["methods"][0]["factory_key"] = "CCA"
            return (json.dumps(contract, indent=2) + "\n").encode("utf-8")

        def non_finite() -> bytes:
            return original.replace(b'"schema_version": 1', b'"schema_version": NaN', 1)

        def escaped_path() -> bytes:
            contract = json.loads(original)
            contract["methods"][0]["manifest"] = "../../outside.manifest.json"
            return (json.dumps(contract, indent=2) + "\n").encode("utf-8")

        def duplicate_path() -> bytes:
            contract = json.loads(original)
            contract["methods"][1]["manifest"] = contract["methods"][0]["manifest"]
            return (json.dumps(contract, indent=2) + "\n").encode("utf-8")

        def schema_invalid_path_characters() -> bytes:
            contract = json.loads(original)
            contract["methods"][0]["factory"]["reference_script"] = "validation/cca release.py"
            return (json.dumps(contract, indent=2) + "\n").encode("utf-8")

        cases = (
            ("duplicate key", duplicate_key, "duplicate JSON key"),
            ("unknown key", unknown_key, "additionalProperties|unknown"),
            ("invalid identifier", invalid_identifier, "JSON Schema pattern|lowercase identifier"),
            ("non-finite", non_finite, "non-finite JSON number"),
            ("path escape", escaped_path, "JSON Schema (pattern|not constraint)|escapes or is not normalized"),
            ("duplicate path", duplicate_path, "duplicate contract path"),
            ("schema-invalid path characters", schema_invalid_path_characters, "JSON Schema pattern"),
        )
        for name, mutate, message in cases:
            with self.subTest(name=name):
                self.contract_path.write_bytes(mutate())
                with self.assertRaisesRegex(codegen.ContractError, message):
                    codegen.load_contract_model(self.root)
                self.contract_path.write_bytes(original)

        schema_path = self.root / codegen.SCHEMA_RELATIVE
        original_schema = schema_path.read_bytes()
        drifted_schema = json.loads(original_schema)
        drifted_schema["$defs"]["identifier"]["pattern"] = ".*"
        schema_path.write_text(json.dumps(drifted_schema, indent=2) + "\n", encoding="utf-8")
        with self.assertRaisesRegex(codegen.ContractError, "schema semantic SHA-256 drifted"):
            codegen.load_contract_model(self.root)
        schema_path.write_bytes(original_schema)

    def test_02_contract_has_exactly_four_sorted_methods(self) -> None:
        model = codegen.load_contract_model(self.root)
        keys = tuple(method["factory_key"] for method in model["methods"])
        self.assertEqual(keys, codegen.EXPECTED_FACTORY_KEYS)
        self.assertEqual(len(keys), 4)
        self.assertEqual(keys, tuple(sorted(keys)))
        self.assertEqual([len(method["capability_requirements"]) for method in model["methods"]], [2, 1, 2, 1])
        self.assertEqual(
            {
                method["factory_key"]: [item["role"] for item in method["capability_requirements"]]
                for method in model["methods"]
            },
            {
                "cca": ["base", "primary"],
                "gsca": ["primary"],
                "ipma": ["base", "primary"],
                "nca": ["primary"],
            },
        )

    def test_03_each_requirement_resolves_one_exact_registry_cell_and_link(self) -> None:
        model = codegen.load_contract_model(self.root)
        registry = codegen.strict_json_file(self.registry_path)
        for method in model["methods"]:
            for requirement in method["capability_requirements"]:
                capabilities = [
                    item
                    for item in registry["capabilities"]
                    if item["capability_id"] == requirement["capability_id"]
                ]
                self.assertEqual(len(capabilities), 1)
                cells = [
                    item
                    for item in capabilities[0]["option_cells"]
                    if item["cell_id"] == requirement["cell_id"]
                ]
                self.assertEqual(len(cells), 1)
                cell = cells[0]
                self.assertEqual(cell["capability_version"], requirement["capability_version"])
                self.assertEqual(
                    cell["qualification_spec"]["links"],
                    [
                        {
                            "registry_schema_version": 2,
                            "capability_id": requirement["capability_id"],
                            "cell_id": requirement["cell_id"],
                            "capability_version": requirement["capability_version"],
                        }
                    ],
                )

        target = registry["capabilities"][0]["option_cells"][0]["qualification_spec"]["links"][0]
        target["capability_version"] = "wrong_v1"
        self.registry_path.write_text(json.dumps(registry, indent=2) + "\n", encoding="utf-8")
        with self.assertRaisesRegex(codegen.ContractError, "identity link does not exactly match"):
            codegen.load_contract_model(self.root)

    def test_04_primary_registry_identity_exactly_matches_method_manifest(self) -> None:
        model = codegen.load_contract_model(self.root)
        for method in model["methods"]:
            primary = next(
                requirement
                for requirement in method["capability_requirements"]
                if requirement["role"] == "primary"
            )
            self.assertEqual(primary["identity_manifest"], method["manifest"])
            self.assertEqual(primary["cell_id"], method["manifest_identity"]["feature_id"])
            self.assertEqual(
                primary["capability_version"], method["manifest_identity"]["method_version"]
            )

        manifest_path = self.root / model["methods"][0]["manifest"]
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["feature"]["method_version"] = "wrong_v1"
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        with self.assertRaisesRegex(codegen.ContractError, "does not exactly match identity manifest"):
            codegen.load_contract_model(self.root)

    def test_05_generated_outputs_do_not_copy_registry_or_manifest_authority_state(self) -> None:
        outputs = codegen.render_outputs(self.root)
        ownership = codegen.strict_json_bytes(outputs[codegen.OUTPUT_RELATIVES[0]], "ownership")
        self.assertIs(ownership["shadow_only"], False)
        self.assertIs(ownership["consumer_adopted"], True)
        self.assertEqual(ownership["adoption_phase"], 2)
        codegen._assert_no_forbidden_keys(ownership)
        for relative, data in outputs.items():
            text = data.decode("utf-8")
            for forbidden in codegen.FORBIDDEN_OUTPUT_KEYS:
                self.assertIsNone(
                    codegen.re.search(rf"\b{codegen.re.escape(forbidden)}\b", text),
                    f"{relative} copied {forbidden}",
                )

    def test_06_render_and_semantic_digest_are_byte_deterministic(self) -> None:
        first = codegen.render_outputs(self.root)
        second = codegen.render_outputs(self.root)
        self.assertEqual(first, second)
        contract = codegen.strict_json_file(self.contract_path)
        expected_digest = codegen.semantic_sha256(contract)
        self.assertEqual(expected_digest, codegen.load_contract_model(self.root)["contract_sha256"])
        for relative, data in first.items():
            self.assertGreater(len(data), 0, relative)
            self.assertTrue(data.endswith(b"\n"), relative)

    def test_07_check_is_read_only_and_detects_a_mutated_output(self) -> None:
        codegen.write_outputs(self.root)
        target = self.root / codegen.OUTPUT_RELATIVES[1]
        target.write_bytes(target.read_bytes() + b"// deliberate drift\n")
        before = {relative: (self.root / relative).read_bytes() for relative in codegen.OUTPUT_RELATIVES}
        mismatches = codegen.check_outputs(self.root)
        after = {relative: (self.root / relative).read_bytes() for relative in codegen.OUTPUT_RELATIVES}
        self.assertEqual(before, after)
        self.assertEqual(mismatches, [f"stale: {codegen.OUTPUT_RELATIVES[1]}"])

    def test_08_write_is_atomic_idempotent_and_limited_to_three_declared_targets(self) -> None:
        before = self.all_files()
        real_replace = os.replace
        with mock.patch.object(codegen.os, "replace", wraps=real_replace) as replace:
            changed = codegen.write_outputs(self.root)
        self.assertEqual(changed, list(codegen.OUTPUT_RELATIVES))
        self.assertEqual(replace.call_count, 3)
        for call in replace.call_args_list:
            temporary, target = (Path(argument) for argument in call.args)
            self.assertEqual(temporary.parent, target.parent)
            self.assertTrue(temporary.name.endswith(".tmp"))
        after = self.all_files()
        self.assertEqual(after - before, set(codegen.OUTPUT_RELATIVES))
        self.assertFalse(any(path.name.endswith(".tmp") for path in self.root.rglob("*")))
        with mock.patch.object(codegen.os, "replace", wraps=real_replace) as replace_again:
            self.assertEqual(codegen.write_outputs(self.root), [])
        replace_again.assert_not_called()

        sentinel = self.root / "symlink-sentinel.ts"
        sentinel.write_bytes(b"sentinel must remain unchanged\n")
        symlink_target = self.root / codegen.OUTPUT_RELATIVES[1]
        symlink_target.unlink()
        simulated_symlink = False
        try:
            symlink_target.symlink_to(sentinel)
        except OSError:
            # Unprivileged Windows sessions may not create symlinks. Preserve the
            # sentinel assertion while simulating the same Path.is_symlink signal.
            symlink_target.write_bytes(b"declared target sentinel must remain unchanged\n")
            simulated_symlink = True
        protected = {
            relative: (self.root / relative).read_bytes()
            for relative in (codegen.OUTPUT_RELATIVES[0], codegen.OUTPUT_RELATIVES[2])
        }
        def reject_redirected_output() -> None:
            with self.assertRaisesRegex(codegen.ContractError, "symlink or reparse point"):
                codegen.write_outputs(self.root)

        if simulated_symlink:
            path_type = type(symlink_target)
            real_is_symlink = path_type.is_symlink

            def report_declared_target_as_symlink(path: Path) -> bool:
                normalized = path.as_posix().casefold()
                if normalized.endswith("/src/domain/generated/establishedmethodcontractsv1.ts"):
                    return True
                return real_is_symlink(path)

            with mock.patch.object(path_type, "is_symlink", new=report_declared_target_as_symlink):
                reject_redirected_output()
            self.assertEqual(
                symlink_target.read_bytes(), b"declared target sentinel must remain unchanged\n"
            )
        else:
            reject_redirected_output()
        self.assertEqual(sentinel.read_bytes(), b"sentinel must remain unchanged\n")
        for relative, expected in protected.items():
            self.assertEqual((self.root / relative).read_bytes(), expected)

    def test_09_factory_and_generated_role_ownership_match_phase_two_contract(self) -> None:
        model = codegen.load_contract_model(self.root)
        factory = codegen._read_factory_methods(self.root)
        self.assertEqual(
            set(factory).intersection(codegen.EXPECTED_FACTORY_KEYS),
            set(codegen.EXPECTED_FACTORY_KEYS),
        )
        expected_roles = {
            "cca": ["base", "primary"],
            "gsca": ["primary"],
            "ipma": ["base", "primary"],
            "nca": ["primary"],
        }
        for method in model["methods"]:
            key = method["factory_key"]
            self.assertEqual(factory[key]["manifest"], method["manifest"])
            self.assertEqual(
                [item["role"] for item in method["capability_requirements"]],
                expected_roles[key],
            )
            primary = next(
                item for item in method["capability_requirements"] if item["role"] == "primary"
            )
            self.assertEqual(method["canonical_table_rules"][0]["owner_options"], [primary["option"]])

    def test_10_typescript_method_import_helper_order_fallback_and_stale_literals_are_strict(self) -> None:
        relative = codegen.PARITY_SOURCES["typescript_method"]
        mutations = (
            (
                "import",
                'import { establishedMethodContractV1 } from "./generated/establishedMethodContractsV1";',
                'import { establishedMethodContractV2 } from "./generated/establishedMethodContractsV1";',
                "TypeScript method generated import",
            ),
            (
                "helper",
                "const contract = establishedMethodContractV1(method, method);",
                'const contract = establishedMethodContractV1(method, "cca");',
                "TypeScript method generated helper",
            ),
            (
                "order",
                "contract.capability_requirements.map((item) => requirement(",
                "[...contract.capability_requirements].reverse().map((item) => requirement(",
                "TypeScript method generated helper",
            ),
            (
                "fallback",
                "if (established) return established;",
                "if (established) return null;",
                "lookup/fallback placement",
            ),
            (
                "fallback placement",
                "  const established = establishedMethodRequirementsV1(method);",
                "  switch (method) {}\n"
                "  const established = establishedMethodRequirementsV1(method);",
                "lookup/fallback placement",
            ),
            (
                "stale literal",
                '  wpls: [requirement("smartpls.wpls", "qpls3.pls.weighted", "weighted_pls")],',
                '  wpls: [requirement("smartpls.wpls", "qpls3.pls.weighted", "weighted_pls")],\n'
                '  gsca: [requirement("smartpls.gsca", "qpls3.gsca.als", "gsca")],',
                "stale gsca simple literal",
            ),
            (
                "stale resolver branch",
                "  const established = establishedMethodRequirementsV1(method);",
                '  if (method === "gsca") return freezeRequirements([]);\n'
                "  const established = establishedMethodRequirementsV1(method);",
                "stale gsca resolver branch",
            ),
        )
        for name, needle, replacement, message in mutations:
            with self.subTest(name=name):
                self.assert_mutation_rejected(relative, needle, replacement, message)

    def test_11_typescript_canonical_import_filter_fallback_and_stale_prefix_are_strict(self) -> None:
        relative = codegen.PARITY_SOURCES["typescript_canonical"]
        mutations = (
            (
                "import",
                "  ESTABLISHED_METHOD_CONTRACTS_V1,",
                "  BROKEN_ESTABLISHED_METHOD_CONTRACTS_V1,",
                "TypeScript canonical generated import",
            ),
            (
                "filter",
                ".filter((item) => item.option === ownerOption)",
                '.filter((item) => item.role === "primary")',
                "TypeScript canonical generated helper",
            ),
            (
                "fallback",
                "if (established) return established;",
                "if (established) return null;",
                "canonical generated lookup/fallback placement",
            ),
            (
                "fallback placement",
                "  const established = establishedCanonicalTableRequirementsV1(tableId);",
                "  return null;\n"
                "  const established = establishedCanonicalTableRequirementsV1(tableId);",
                "canonical generated lookup/fallback placement",
            ),
            (
                "stale prefix",
                "  const established = establishedCanonicalTableRequirementsV1(tableId);",
                '  if (tableId.startsWith("gsca_")) {\n'
                '    return [requirement("smartpls.gsca", "qpls3.gsca.als", "gsca")];\n'
                "  }\n"
                "  const established = establishedCanonicalTableRequirementsV1(tableId);",
                "stale gsca prefix branch",
            ),
        )
        for name, needle, replacement, message in mutations:
            with self.subTest(name=name):
                self.assert_mutation_rejected(relative, needle, replacement, message)

    def test_12_rust_core_generated_module_import_is_strict(self) -> None:
        self.assert_mutation_rejected(
            codegen.PARITY_SOURCES["rust_core_module"],
            "        established_method_contract_v1,",
            "        established_method_contract_v2,",
            "Rust core generated module/export",
        )

    def test_13_rust_cli_lookup_helper_role_order_and_filter_are_strict(self) -> None:
        relative = codegen.PARITY_SOURCES["rust_cli"]
        mutations = (
            (
                "lookup import",
                "qpls_core::generated::established_method_contract_v1(method.as_str(), config_kind)",
                "qpls_core::legacy_generated::established_method_contract_v1(method.as_str(), config_kind)",
                "Rust CLI generated lookup/helper/order/filter",
            ),
            (
                "role order",
                'const ESTABLISHED_CLI_REQUIREMENT_ROLE_ORDER_V1: [&str; 2] = ["primary", "base"];',
                'const ESTABLISHED_CLI_REQUIREMENT_ROLE_ORDER_V1: [&str; 2] = ["base", "primary"];',
                "Rust CLI generated helper start marker|lookup/helper/order/filter",
            ),
            (
                "role filter",
                ".filter(|requirement| requirement.role == role)",
                ".filter(|requirement| requirement.role != role)",
                "Rust CLI generated lookup/helper/order/filter",
            ),
            (
                "helper body",
                "    Ok(())\n}\n\nfn required_cli_capability_cells(",
                "    required_cells.reverse();\n    Ok(())\n}\n\nfn required_cli_capability_cells(",
                "Rust CLI generated lookup/helper/order/filter",
            ),
        )
        for name, needle, replacement, message in mutations:
            with self.subTest(name=name):
                self.assert_mutation_rejected(relative, needle, replacement, message)

    def test_14_rust_cli_four_arms_dynamic_fallback_and_stale_literal_are_strict(self) -> None:
        relative = codegen.PARITY_SOURCES["rust_cli"]
        adopted_call = """            push_generated_established_cli_capability_cells(
                &mut required_cells,
                method,
                config.kind(),
            )?;"""
        mutations = (
            (
                "missing branch",
                "        (AnalysisMethod::Gsca, MethodConfig::Gsca) => {\n" + adopted_call,
                "        (AnalysisMethod::Gsca, MethodConfig::Gsca) => {\n"
                + adopted_call.replace(
                    "push_generated_established_cli_capability_cells",
                    "push_missing_established_cli_capability_cells",
                ),
                "Rust CLI adopted gsca arm|route exactly",
            ),
            (
                "extra branch",
                "        (AnalysisMethod::PlsSampleSizePower, MethodConfig::PlsSampleSizePower(_)) => {",
                "        (AnalysisMethod::PlsSampleSizePower, MethodConfig::PlsSampleSizePower(_)) => {\n"
                + adopted_call,
                "route exactly",
            ),
            (
                "extra target branch without helper",
                "        (AnalysisMethod::Cca, MethodConfig::Cca) => {\n"
                + adopted_call
                + "\n        }",
                "        (AnalysisMethod::Cca, MethodConfig::Cca) => {\n"
                + adopted_call
                + "\n        }\n"
                "        (AnalysisMethod::Cca, _) => {\n"
                "            return Err(unmapped_cli_capability_error(\n"
                "                method,\n"
                "                config.kind(),\n"
                "                CliCapabilityMappingFailure::UnmappedMethodConfig,\n"
                "            ));\n"
                "        }",
                "adopted cca method branch must occur exactly once",
            ),
            (
                "extra target config branch without helper",
                "        (AnalysisMethod::Cca, MethodConfig::Cca) => {",
                "        (candidate, MethodConfig::Cca) => {\n"
                "            return Err(unmapped_cli_capability_error(\n"
                "                candidate,\n"
                "                config.kind(),\n"
                "                CliCapabilityMappingFailure::UnmappedMethodConfig,\n"
                "            ));\n"
                "        }\n"
                "        (AnalysisMethod::Cca, MethodConfig::Cca) => {",
                "adopted cca config branch must occur exactly once",
            ),
            (
                "stale literal",
                "        (AnalysisMethod::Gsca, MethodConfig::Gsca) => {\n"
                + adopted_call,
                """        (AnalysisMethod::Gsca, MethodConfig::Gsca) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.gsca",
                "qpls3.gsca.als",
                "gsca_als_v2",
            );""",
                "Rust CLI adopted gsca arm|stale gsca primary literal",
            ),
            (
                "dynamic fallback",
                """        (AnalysisMethod::Legacy, MethodConfig::Legacy) => {
            return Err(unmapped_cli_capability_error(
                method,
                config.kind(),
                CliCapabilityMappingFailure::UnmappedMethodConfig,
            ));
        }""",
                """        (AnalysisMethod::Legacy, MethodConfig::Legacy) => {
            return Err(unmapped_cli_capability_error(
                method,
                config.kind(),
                CliCapabilityMappingFailure::MethodConfigMismatch,
            ));
        }""",
                "legacy dynamic fallback",
            ),
            (
                "dynamic preflight prefix",
                "    let method = recipe.settings.method;\n"
                "    let Some(config) = recipe.method_config.as_ref() else {",
                "    let method = recipe.settings.method;\n"
                "    let mut required_cells = Vec::new();\n"
                "    let Some(config) = recipe.method_config.as_ref() else {",
                "missing/mismatch dynamic fallback preflight drifted",
            ),
            (
                "missing-config error bytes",
                '            "<missing>",',
                '            "<mis sing>",',
                "missing-config error bytes drifted",
            ),
            (
                "wildcard fallback",
                """        _ => {
            return Err(unmapped_cli_capability_error(
                method,
                config.kind(),
                CliCapabilityMappingFailure::UnmappedMethodConfig,
            ));
        }""",
                """        _ => {
            return Err(unmapped_cli_capability_error(
                method,
                config.kind(),
                CliCapabilityMappingFailure::MethodConfigMismatch,
            ));
        }""",
                "wildcard dynamic fallback",
            ),
        )
        for name, needle, replacement, message in mutations:
            with self.subTest(name=name):
                self.assert_mutation_rejected(relative, needle, replacement, message)

    def test_15_all_ownership_paths_exist_and_canonical_prefixes_do_not_overlap(self) -> None:
        codegen.write_outputs(self.root)
        ownership_path = self.root / codegen.OUTPUT_RELATIVES[0]
        ownership = codegen.strict_json_file(ownership_path)
        self.assertEqual(ownership["adoption_phase"], 2)
        self.assertIs(ownership["consumer_adopted"], True)
        self.assertIs(ownership["shadow_only"], False)
        self.assertEqual(ownership["consumer_sources"], codegen.PARITY_SOURCES)
        self.assertNotIn("parity_sources", ownership)
        self.assertEqual(len(ownership["input_paths"]), len(set(ownership["input_paths"])))
        self.assertEqual(len(ownership["generated_targets"]), len(set(ownership["generated_targets"])))
        for relative in ownership["input_paths"] + ownership["generated_targets"]:
            self.assertTrue((self.root / relative).is_file(), relative)
        prefixes = [
            method["canonical_table_rules"][0]["value"] for method in ownership["methods"]
        ]
        for index, left in enumerate(prefixes):
            for right in prefixes[index + 1 :]:
                self.assertFalse(left.startswith(right) or right.startswith(left), (left, right))


if __name__ == "__main__":
    unittest.main()
