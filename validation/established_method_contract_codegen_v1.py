#!/usr/bin/env python3
"""Validate and generate the shadow-only established-method contract artifacts."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_RELATIVE = "validation/method_contracts/established_methods_v1.schema.json"
CONTRACT_RELATIVE = "validation/method_contracts/established_methods_v1.json"
GENERATOR_RELATIVE = "validation/established_method_contract_codegen_v1.py"
REGISTRY_RELATIVE = "validation/capabilities/capability_registry_v2.json"
FACTORY_RELATIVE = "validation/established_method_factory_common.py"
PARITY_SOURCES = {
    "python_factory": FACTORY_RELATIVE,
    "rust_cli": "crates/qpls-cli/src/main.rs",
    "rust_core_module": "crates/qpls-core/src/lib.rs",
    "typescript_canonical": "src/native/nativeCanonicalResultDocumentV2.ts",
    "typescript_method": "src/domain/methodCapabilityRegistryV2.ts",
}
OUTPUT_RELATIVES = (
    "validation/method_contracts/generated/established_method_ownership_v1.json",
    "src/domain/generated/establishedMethodContractsV1.ts",
    "crates/qpls-core/src/generated/established_method_contracts_v1.rs",
)
EXPECTED_FACTORY_KEYS = ("cca", "gsca", "ipma", "nca")
EXPECTED_SCHEMA_ID = "https://quickpls.local/schemas/established_methods_v1.schema.json"
EXPECTED_SCHEMA_SHA256 = "f21a3e599cfc0f3ecf7cfc792e033235bff993cd0465dc1288e032ce7f18bfbd"
EXPECTED_CONTRACT_ID = "quickpls.established_method_integration.v1"
IDENTIFIER = re.compile(r"^[a-z][a-z0-9_]*$")
DOTTED_IDENTIFIER = re.compile(r"^[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+$")
PREFIX = re.compile(r"^[a-z][a-z0-9_]*_$")
JSON_POINTER = re.compile(r"^/(?:[^~/]|~[01])+(?:/(?:[^~/]|~[01])+)*$")
REPOSITORY_PATH = re.compile(r"^[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*$")
FORBIDDEN_OUTPUT_KEYS = frozenset(
    {
        "catalogue_snapshot_date",
        "coverage",
        "coverage_state",
        "declared_state",
        "evidence",
        "evidence_state",
        "frozen_on",
        "generated_at",
        "qualification",
        "qualification_state",
        "receipts",
        "state",
        "surface",
        "target_state",
    }
)


class ContractError(ValueError):
    """Raised when a contract or one of its authoritative inputs is invalid."""


def _reject_duplicate_pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in items:
        if key in result:
            raise ContractError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_non_finite(value: str) -> None:
    raise ContractError(f"non-finite JSON number: {value}")


def strict_json_bytes(data: bytes, label: str) -> Any:
    try:
        text = data.decode("utf-8")
        return json.loads(
            text,
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=_reject_non_finite,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ContractError(f"invalid JSON in {label}: {error}") from error


def strict_json_file(path: Path) -> Any:
    try:
        return strict_json_bytes(path.read_bytes(), str(path))
    except OSError as error:
        raise ContractError(f"cannot read {path}: {error}") from error


def canonical_json_bytes(value: Any) -> bytes:
    try:
        return json.dumps(
            value,
            allow_nan=False,
            ensure_ascii=False,
            separators=(",", ":"),
            sort_keys=True,
        ).encode("utf-8")
    except (TypeError, ValueError) as error:
        raise ContractError(f"value is not canonical JSON: {error}") from error


def semantic_sha256(value: Any) -> str:
    return hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def _expect_dict(value: Any, field: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ContractError(f"{field} must be an object")
    return value


def _expect_list(value: Any, field: str) -> list[Any]:
    if not isinstance(value, list):
        raise ContractError(f"{field} must be an array")
    return value


def _expect_string(value: Any, field: str) -> str:
    if not isinstance(value, str):
        raise ContractError(f"{field} must be a string")
    return value


def _expect_integer(value: Any, field: str) -> int:
    if type(value) is not int:
        raise ContractError(f"{field} must be an integer")
    return value


def _expect_exact_keys(value: dict[str, Any], expected: Iterable[str], field: str) -> None:
    expected_set = set(expected)
    actual_set = set(value)
    missing = sorted(expected_set - actual_set)
    unknown = sorted(actual_set - expected_set)
    if missing or unknown:
        details: list[str] = []
        if missing:
            details.append(f"missing={missing}")
        if unknown:
            details.append(f"unknown={unknown}")
        raise ContractError(f"{field} has invalid keys ({', '.join(details)})")


def _validate_identifier(value: Any, field: str) -> str:
    text = _expect_string(value, field)
    if not IDENTIFIER.fullmatch(text):
        raise ContractError(f"{field} is not a lowercase identifier: {text!r}")
    return text


def _validate_dotted_identifier(value: Any, field: str) -> str:
    text = _expect_string(value, field)
    if not DOTTED_IDENTIFIER.fullmatch(text):
        raise ContractError(f"{field} is not a dotted lowercase identifier: {text!r}")
    return text


def _validate_relative_path(value: Any, field: str) -> str:
    text = _expect_string(value, field)
    if not text or not REPOSITORY_PATH.fullmatch(text):
        raise ContractError(f"{field} must be a normalized repository-relative POSIX path")
    path = PurePosixPath(text)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ContractError(f"{field} escapes or is not normalized: {text!r}")
    if path.as_posix() != text:
        raise ContractError(f"{field} is not normalized: {text!r}")
    return text


def _contained_path(root: Path, relative: str, *, must_exist: bool) -> Path:
    root_resolved = root.resolve(strict=True)
    posix = PurePosixPath(relative)
    lexical_candidate = root_resolved.joinpath(*posix.parts)
    if not must_exist:
        current = root_resolved
        for part in posix.parts:
            current = current / part
            if current.is_symlink():
                raise ContractError(f"output path contains a symlink or reparse point: {relative}")
            if current.exists():
                try:
                    attributes = getattr(current.lstat(), "st_file_attributes", 0)
                except OSError as error:
                    raise ContractError(f"cannot inspect output path component {current}: {error}") from error
                if attributes & 0x400:
                    raise ContractError(f"output path contains a symlink or reparse point: {relative}")
        return lexical_candidate
    candidate = lexical_candidate.resolve(strict=True)
    try:
        candidate.relative_to(root_resolved)
    except ValueError as error:
        raise ContractError(f"path escapes repository root: {relative}") from error
    if must_exist and not candidate.is_file():
        raise ContractError(f"required input is not a file: {relative}")
    return candidate


def _input_file(root: Path, relative: str) -> Path:
    _validate_relative_path(relative, "input path")
    try:
        return _contained_path(root, relative, must_exist=True)
    except FileNotFoundError as error:
        raise ContractError(f"required input does not exist: {relative}") from error


def declared_output_paths(root: Path) -> tuple[Path, ...]:
    if len(set(OUTPUT_RELATIVES)) != len(OUTPUT_RELATIVES):
        raise ContractError("duplicate declared output path")
    paths: list[Path] = []
    for relative in OUTPUT_RELATIVES:
        _validate_relative_path(relative, "output path")
        paths.append(_contained_path(root, relative, must_exist=False))
    return tuple(paths)


def _validate_schema(schema: Any) -> dict[str, Any]:
    value = _expect_dict(schema, "schema")
    _expect_exact_keys(
        value,
        {
            "$schema",
            "$id",
            "$defs",
            "additionalProperties",
            "properties",
            "required",
            "title",
            "type",
        },
        "schema",
    )
    if value["$schema"] != "https://json-schema.org/draft/2020-12/schema":
        raise ContractError("schema draft must be JSON Schema 2020-12")
    if value["$id"] != EXPECTED_SCHEMA_ID:
        raise ContractError("unexpected schema id")
    actual_sha256 = semantic_sha256(value)
    if actual_sha256 != EXPECTED_SCHEMA_SHA256:
        raise ContractError(
            f"schema semantic SHA-256 drifted: expected {EXPECTED_SCHEMA_SHA256}, found {actual_sha256}"
        )
    if value["type"] != "object" or value["additionalProperties"] is not False:
        raise ContractError("schema root must be a closed object")
    required = ["$schema", "schema_version", "contract_id", "registry", "methods"]
    if value["required"] != required:
        raise ContractError("schema root required keys drifted")
    definitions = _expect_dict(value["$defs"], "schema.$defs")
    for name in ("method", "runtime", "capabilityRequirement", "canonicalTableRule", "factory"):
        definition = _expect_dict(definitions.get(name), f"schema.$defs.{name}")
        if definition.get("type") != "object" or definition.get("additionalProperties") is not False:
            raise ContractError(f"schema.$defs.{name} must be a closed object")
    return value


def _json_schema_equal(left: Any, right: Any) -> bool:
    return canonical_json_bytes(left) == canonical_json_bytes(right)


def _resolve_local_schema_reference(reference: Any, root_schema: dict[str, Any]) -> dict[str, Any]:
    text = _expect_string(reference, "schema.$ref")
    if not text.startswith("#/"):
        raise ContractError(f"schema uses a non-local $ref: {text!r}")
    current: Any = root_schema
    for raw_token in text[2:].split("/"):
        token = raw_token.replace("~1", "/").replace("~0", "~")
        if not isinstance(current, dict) or token not in current:
            raise ContractError(f"schema $ref does not resolve: {text!r}")
        current = current[token]
    return _expect_dict(current, f"schema reference {text}")


def _validate_json_schema_instance(
    instance: Any,
    schema: Any,
    root_schema: dict[str, Any],
    field: str,
) -> None:
    rule = _expect_dict(schema, f"schema rule for {field}")
    if "$ref" in rule:
        if set(rule) != {"$ref"}:
            raise ContractError(f"schema $ref for {field} must not have sibling constraints")
        _validate_json_schema_instance(
            instance,
            _resolve_local_schema_reference(rule["$ref"], root_schema),
            root_schema,
            field,
        )
        return

    expected_type = rule.get("type")
    if expected_type is not None:
        type_checks = {
            "array": lambda value: isinstance(value, list),
            "integer": lambda value: type(value) is int,
            "object": lambda value: isinstance(value, dict),
            "string": lambda value: isinstance(value, str),
        }
        if expected_type not in type_checks:
            raise ContractError(f"schema uses unsupported type {expected_type!r} at {field}")
        if not type_checks[expected_type](instance):
            raise ContractError(f"{field} violates JSON Schema type {expected_type}")

    if "const" in rule and not _json_schema_equal(instance, rule["const"]):
        raise ContractError(f"{field} violates JSON Schema const {rule['const']!r}")
    if "enum" in rule:
        choices = _expect_list(rule["enum"], f"schema enum for {field}")
        if not any(_json_schema_equal(instance, choice) for choice in choices):
            raise ContractError(f"{field} violates JSON Schema enum {choices!r}")
    if "pattern" in rule and isinstance(instance, str):
        pattern = _expect_string(rule["pattern"], f"schema pattern for {field}")
        try:
            matched = re.search(pattern, instance) is not None
        except re.error as error:
            raise ContractError(f"schema has invalid pattern for {field}: {error}") from error
        if not matched:
            raise ContractError(f"{field} violates JSON Schema pattern {pattern!r}")
    if "not" in rule:
        try:
            _validate_json_schema_instance(instance, rule["not"], root_schema, field)
        except ContractError:
            pass
        else:
            raise ContractError(f"{field} violates JSON Schema not constraint")

    if isinstance(instance, dict):
        required = rule.get("required", [])
        required_keys = _expect_list(required, f"schema required for {field}")
        missing = [key for key in required_keys if key not in instance]
        if missing:
            raise ContractError(f"{field} violates JSON Schema required keys: {missing}")
        properties = _expect_dict(rule.get("properties", {}), f"schema properties for {field}")
        if rule.get("additionalProperties") is False:
            unknown = sorted(set(instance) - set(properties))
            if unknown:
                raise ContractError(f"{field} violates JSON Schema additionalProperties: {unknown}")
        for key, value in instance.items():
            if key in properties:
                _validate_json_schema_instance(value, properties[key], root_schema, f"{field}.{key}")

    if isinstance(instance, list):
        if "minItems" in rule and len(instance) < _expect_integer(rule["minItems"], "schema.minItems"):
            raise ContractError(f"{field} violates JSON Schema minItems")
        if "maxItems" in rule and len(instance) > _expect_integer(rule["maxItems"], "schema.maxItems"):
            raise ContractError(f"{field} violates JSON Schema maxItems")
        if rule.get("uniqueItems") is True:
            identities = [canonical_json_bytes(item) for item in instance]
            if len(identities) != len(set(identities)):
                raise ContractError(f"{field} violates JSON Schema uniqueItems")
        if "items" in rule:
            for index, item in enumerate(instance):
                _validate_json_schema_instance(item, rule["items"], root_schema, f"{field}[{index}]")


def _validate_factory(factory: Any, field: str, root: Path) -> dict[str, Any]:
    value = _expect_dict(factory, field)
    _expect_exact_keys(
        value,
        {
            "boundary_test_filter",
            "output",
            "persistence_test_filter",
            "reference_report",
            "reference_script",
            "reference_version_pointer",
            "simulation_test_filter",
        },
        field,
    )
    output = _validate_identifier(value["output"], f"{field}.output")
    script = _validate_relative_path(value["reference_script"], f"{field}.reference_script")
    report = _validate_relative_path(value["reference_report"], f"{field}.reference_report")
    if not script.startswith("validation/") or not script.endswith(".py"):
        raise ContractError(f"{field}.reference_script must be a validation Python file")
    expected_report = f"validation/results/method_factory/{output}/independent_reference.json"
    if report != expected_report:
        raise ContractError(f"{field}.reference_report must be {expected_report}")
    _input_file(root, script)
    _input_file(root, report)
    pointer = _expect_string(value["reference_version_pointer"], f"{field}.reference_version_pointer")
    if not JSON_POINTER.fullmatch(pointer):
        raise ContractError(f"{field}.reference_version_pointer is not a strict JSON pointer")
    for key in ("simulation_test_filter", "boundary_test_filter", "persistence_test_filter"):
        _validate_identifier(value[key], f"{field}.{key}")
    return value


def _validate_contract(contract: Any, root: Path) -> dict[str, Any]:
    value = _expect_dict(contract, "contract")
    _expect_exact_keys(
        value,
        {"$schema", "schema_version", "contract_id", "registry", "methods"},
        "contract",
    )
    if value["$schema"] != "established_methods_v1.schema.json":
        raise ContractError("contract $schema must name the adjacent v1 schema")
    if _expect_integer(value["schema_version"], "contract.schema_version") != 1:
        raise ContractError("contract.schema_version must be 1")
    if value["contract_id"] != EXPECTED_CONTRACT_ID:
        raise ContractError("unexpected contract id")
    registry = _validate_relative_path(value["registry"], "contract.registry")
    if registry != REGISTRY_RELATIVE:
        raise ContractError(f"contract.registry must be {REGISTRY_RELATIVE}")
    _input_file(root, registry)

    methods = _expect_list(value["methods"], "contract.methods")
    keys: list[str] = []
    prefixes: list[tuple[str, str]] = []
    owned_paths: dict[str, str] = {}
    for index, method_value in enumerate(methods):
        field = f"contract.methods[{index}]"
        method = _expect_dict(method_value, field)
        _expect_exact_keys(
            method,
            {
                "canonical_table_rules",
                "capability_requirements",
                "factory",
                "factory_key",
                "manifest",
                "runtime",
            },
            field,
        )
        key = _validate_identifier(method["factory_key"], f"{field}.factory_key")
        keys.append(key)

        manifest = _validate_relative_path(method["manifest"], f"{field}.manifest")
        if not manifest.startswith("validation/methods/") or not manifest.endswith(".manifest.json"):
            raise ContractError(f"{field}.manifest must be a method manifest")
        _input_file(root, manifest)

        runtime = _expect_dict(method["runtime"], f"{field}.runtime")
        runtime_keys = {
            "analysis_method",
            "method_config_kind",
            "result_discriminator",
            "workbench_kind",
        }
        _expect_exact_keys(runtime, runtime_keys, f"{field}.runtime")
        for runtime_key in sorted(runtime_keys):
            actual = _validate_identifier(runtime[runtime_key], f"{field}.runtime.{runtime_key}")
            if actual != key:
                raise ContractError(f"{field}.runtime.{runtime_key} must equal factory_key")

        requirements = _expect_list(method["capability_requirements"], f"{field}.capability_requirements")
        if not 1 <= len(requirements) <= 2:
            raise ContractError(f"{field}.capability_requirements must contain one or two entries")
        roles: list[str] = []
        requirement_keys: set[tuple[str, str, str]] = set()
        options: list[str] = []
        for requirement_index, requirement_value in enumerate(requirements):
            requirement_field = f"{field}.capability_requirements[{requirement_index}]"
            requirement = _expect_dict(requirement_value, requirement_field)
            _expect_exact_keys(
                requirement,
                {"capability_id", "cell_id", "option", "role"},
                requirement_field,
            )
            capability_id = _validate_dotted_identifier(
                requirement["capability_id"], f"{requirement_field}.capability_id"
            )
            cell_id = _validate_dotted_identifier(requirement["cell_id"], f"{requirement_field}.cell_id")
            option = _validate_identifier(requirement["option"], f"{requirement_field}.option")
            role = _expect_string(requirement["role"], f"{requirement_field}.role")
            if role not in {"base", "primary"}:
                raise ContractError(f"{requirement_field}.role must be base or primary")
            identity = (capability_id, cell_id, option)
            if identity in requirement_keys:
                raise ContractError(f"duplicate capability requirement: {identity}")
            requirement_keys.add(identity)
            roles.append(role)
            options.append(option)
        expected_roles = ["primary"] if len(requirements) == 1 else ["base", "primary"]
        if roles != expected_roles:
            raise ContractError(f"{field}.capability_requirements roles must be ordered {expected_roles}")
        primary = requirements[-1]
        if primary["option"] != key:
            raise ContractError(f"{field} primary option must equal factory_key")

        rules = _expect_list(method["canonical_table_rules"], f"{field}.canonical_table_rules")
        if len(rules) != 1:
            raise ContractError(f"{field}.canonical_table_rules must contain exactly one prefix rule")
        rule = _expect_dict(rules[0], f"{field}.canonical_table_rules[0]")
        _expect_exact_keys(rule, {"match", "owner_options", "value"}, f"{field}.canonical_table_rules[0]")
        if rule["match"] != "prefix":
            raise ContractError(f"{field}.canonical_table_rules[0].match must be prefix")
        prefix = _expect_string(rule["value"], f"{field}.canonical_table_rules[0].value")
        if not PREFIX.fullmatch(prefix):
            raise ContractError(f"{field}.canonical_table_rules[0].value is not a table prefix")
        owner_options = _expect_list(rule["owner_options"], f"{field}.canonical_table_rules[0].owner_options")
        if owner_options != [primary["option"]]:
            raise ContractError(f"{field} canonical owner_options must contain the primary option")
        prefixes.append((key, prefix))

        factory = _validate_factory(method["factory"], f"{field}.factory", root)
        for label, path in (
            (f"{key}.manifest", manifest),
            (f"{key}.reference_script", factory["reference_script"]),
            (f"{key}.reference_report", factory["reference_report"]),
        ):
            normalized_path = path.casefold()
            previous = owned_paths.get(normalized_path)
            if previous is not None:
                raise ContractError(f"duplicate contract path {path!r}: {previous} and {label}")
            owned_paths[normalized_path] = label

    if tuple(keys) != EXPECTED_FACTORY_KEYS:
        raise ContractError(
            f"contract methods must be exactly sorted {list(EXPECTED_FACTORY_KEYS)}, found {keys}"
        )
    for index, (left_key, left) in enumerate(prefixes):
        for right_key, right in prefixes[index + 1 :]:
            if left.startswith(right) or right.startswith(left):
                raise ContractError(
                    f"canonical prefixes overlap: {left_key}={left!r}, {right_key}={right!r}"
                )
    return value


def _find_unique(items: Sequence[Any], key: str, expected: str, field: str) -> dict[str, Any]:
    matches = [item for item in items if isinstance(item, dict) and item.get(key) == expected]
    if len(matches) != 1:
        raise ContractError(f"{field} must resolve exactly once for {key}={expected!r}, found {len(matches)}")
    return matches[0]


def _manifest_identity(root: Path, relative: str) -> dict[str, str]:
    manifest = _expect_dict(strict_json_file(_input_file(root, relative)), f"manifest {relative}")
    feature = _expect_dict(manifest.get("feature"), f"manifest {relative}.feature")
    feature_id = _validate_dotted_identifier(feature.get("id"), f"manifest {relative}.feature.id")
    method_version = _validate_identifier(
        feature.get("method_version"), f"manifest {relative}.feature.method_version"
    )
    return {"path": relative, "feature_id": feature_id, "method_version": method_version}


def _resolve_requirement(
    root: Path,
    registry_schema_version: int,
    capabilities: list[Any],
    requirement: dict[str, Any],
) -> dict[str, Any]:
    capability_id = requirement["capability_id"]
    cell_id = requirement["cell_id"]
    capability = _find_unique(capabilities, "capability_id", capability_id, "registry capability")
    cells = _expect_list(capability.get("option_cells"), f"registry {capability_id}.option_cells")
    cell = _find_unique(cells, "cell_id", cell_id, f"registry {capability_id} option cell")
    if cell.get("capability_id") != capability_id:
        raise ContractError(f"registry cell {cell_id} repeats a different capability_id")
    capability_version = _validate_identifier(
        cell.get("capability_version"), f"registry cell {cell_id}.capability_version"
    )
    specification = _expect_dict(cell.get("qualification_spec"), f"registry cell {cell_id}.qualification_spec")
    references = _expect_list(specification.get("references"), f"registry cell {cell_id}.references")
    if len(references) != 1:
        raise ContractError(f"registry cell {cell_id} must have exactly one identity manifest")
    identity_manifest = _validate_relative_path(references[0], f"registry cell {cell_id}.references[0]")
    links = _expect_list(specification.get("links"), f"registry cell {cell_id}.links")
    if len(links) != 1:
        raise ContractError(f"registry cell {cell_id} must have exactly one identity link")
    link = _expect_dict(links[0], f"registry cell {cell_id}.links[0]")
    _expect_exact_keys(
        link,
        {"capability_id", "capability_version", "cell_id", "registry_schema_version"},
        f"registry cell {cell_id}.links[0]",
    )
    expected_link = {
        "registry_schema_version": registry_schema_version,
        "capability_id": capability_id,
        "cell_id": cell_id,
        "capability_version": capability_version,
    }
    if link != expected_link:
        raise ContractError(f"registry cell {cell_id} identity link does not exactly match the cell")
    identity = _manifest_identity(root, identity_manifest)
    if identity["feature_id"] != cell_id or identity["method_version"] != capability_version:
        raise ContractError(f"registry cell {cell_id} does not exactly match identity manifest {identity_manifest}")
    return {
        "registry_schema_version": registry_schema_version,
        "capability_id": capability_id,
        "cell_id": cell_id,
        "capability_version": capability_version,
        "option": requirement["option"],
        "role": requirement["role"],
        "identity_manifest": identity_manifest,
    }


def _ast_literal(node: ast.AST) -> Any:
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, ast.List):
        return [_ast_literal(item) for item in node.elts]
    if isinstance(node, ast.Tuple):
        return tuple(_ast_literal(item) for item in node.elts)
    if isinstance(node, ast.Dict):
        result: dict[Any, Any] = {}
        for key_node, value_node in zip(node.keys, node.values):
            if key_node is None:
                raise ContractError("factory METHODS cannot use dictionary expansion")
            key = _ast_literal(key_node)
            if key in result:
                raise ContractError(f"duplicate factory METHODS key: {key!r}")
            result[key] = _ast_literal(value_node)
        return result
    if (
        isinstance(node, ast.Attribute)
        and isinstance(node.value, ast.Name)
        and node.value.id == "sys"
        and node.attr == "executable"
    ):
        return "<sys.executable>"
    raise ContractError(f"factory METHODS contains a non-literal expression: {ast.dump(node, include_attributes=False)}")


def _read_factory_methods(root: Path) -> dict[str, dict[str, Any]]:
    path = _input_file(root, FACTORY_RELATIVE)
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (OSError, SyntaxError, UnicodeError) as error:
        raise ContractError(f"cannot parse factory mapping {FACTORY_RELATIVE}: {error}") from error
    assignments: list[ast.AST] = []
    for node in tree.body:
        if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name) and node.target.id == "METHODS":
            if node.value is not None:
                assignments.append(node.value)
        elif isinstance(node, ast.Assign) and any(
            isinstance(target, ast.Name) and target.id == "METHODS" for target in node.targets
        ):
            assignments.append(node.value)
    if len(assignments) != 1:
        raise ContractError(f"factory source must define METHODS exactly once, found {len(assignments)}")
    methods = _ast_literal(assignments[0])
    if not isinstance(methods, dict):
        raise ContractError("factory METHODS must be a dictionary literal")
    return methods


def _pointer_tokens(pointer: str) -> list[str]:
    return [token.replace("~1", "/").replace("~0", "~") for token in pointer[1:].split("/")]


def _verify_factory_parity(methods: list[dict[str, Any]], root: Path) -> None:
    existing = _read_factory_methods(root)
    for method in methods:
        key = method["factory_key"]
        current = _expect_dict(existing.get(key), f"factory METHODS[{key!r}]")
        factory = method["factory"]
        expected_values = {
            "manifest": method["manifest"],
            "feature_id": method["manifest_identity"]["feature_id"],
            "method_version": method["manifest_identity"]["method_version"],
            "output": factory["output"],
            "reference_report": factory["reference_report"],
            "reference_pointer": _pointer_tokens(factory["reference_version_pointer"]),
            "simulation_filter": factory["simulation_test_filter"],
            "boundary_filter": factory["boundary_test_filter"],
            "persistence_filter": factory["persistence_test_filter"],
        }
        for field, expected in expected_values.items():
            if current.get(field) != expected:
                raise ContractError(
                    f"factory parity failed for {key}.{field}: contract={expected!r}, existing={current.get(field)!r}"
                )
        command = current.get("reference_command")
        if command != ["<sys.executable>", factory["reference_script"]]:
            raise ContractError(f"factory parity failed for {key}.reference_command")


def _read_utf8(root: Path, relative: str) -> str:
    path = _input_file(root, relative)
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ContractError(f"cannot read UTF-8 source {relative}: {error}") from error


def _literal_call_tuple_list(text: str, function: str) -> list[tuple[str, str, str]]:
    pattern = re.compile(
        rf"{re.escape(function)}\(\s*(?:[^,]+,\s*)?\"([^\"]+)\"\s*,\s*\"([^\"]+)\"\s*,\s*\"([^\"]+)\"",
        re.MULTILINE,
    )
    return [match.groups() for match in pattern.finditer(text)]


def _literal_call_tuples(text: str, function: str) -> set[tuple[str, str, str]]:
    return set(_literal_call_tuple_list(text, function))


def _unique_match(pattern: str, text: str, label: str, *, flags: int = 0) -> re.Match[str]:
    matches = list(re.finditer(pattern, text, flags))
    if len(matches) != 1:
        raise ContractError(f"{label} must resolve exactly once, found {len(matches)}")
    return matches[0]


def _source_section(text: str, start: str, end: str, label: str) -> str:
    if text.count(start) != 1:
        raise ContractError(f"{label} start marker must occur exactly once")
    start_index = text.index(start)
    end_index = text.find(end, start_index + len(start))
    if end_index < 0:
        raise ContractError(f"{label} end marker is missing")
    return text[start_index:end_index]


def _compact_source(text: str) -> str:
    without_line_comments = re.sub(r"//[^\r\n]*", "", text)
    return re.sub(r"\s+", "", without_line_comments)


def _expect_compact_source(actual: str, expected: str, label: str) -> None:
    compact_actual = _compact_source(actual)
    compact_expected = _compact_source(expected)
    if compact_actual != compact_expected:
        raise ContractError(f"{label} drifted from the adopted generated-contract consumer shape")


def _primary_ts_tuples(methods: list[dict[str, Any]]) -> dict[str, tuple[str, str, str]]:
    return {
        method["factory_key"]: (
            primary["capability_id"],
            primary["cell_id"],
            primary["option"],
        )
        for method in methods
        for primary in method["capability_requirements"]
        if primary["role"] == "primary"
    }


def _primary_rust_tuples(methods: list[dict[str, Any]]) -> dict[str, tuple[str, str, str]]:
    return {
        method["factory_key"]: (
            primary["capability_id"],
            primary["cell_id"],
            primary["capability_version"],
        )
        for method in methods
        for primary in method["capability_requirements"]
        if primary["role"] == "primary"
    }


def _verify_generated_contract_role_ownership(methods: list[dict[str, Any]]) -> None:
    expected_roles = {
        "cca": ["base", "primary"],
        "gsca": ["primary"],
        "ipma": ["base", "primary"],
        "nca": ["primary"],
    }
    actual = {
        method["factory_key"]: [item["role"] for item in method["capability_requirements"]]
        for method in methods
    }
    if actual != expected_roles:
        raise ContractError(f"generated TypeScript method role ownership drifted: {actual}")
    for method in methods:
        primary = next(item for item in method["capability_requirements"] if item["role"] == "primary")
        for rule in method["canonical_table_rules"]:
            if rule["owner_options"] != [primary["option"]]:
                raise ContractError(
                    f"generated canonical ownership for {method['factory_key']} must be primary-only"
                )


def _verify_typescript_method_adoption(methods: list[dict[str, Any]], source: str) -> None:
    import_pattern = (
        r'import\s*\{\s*establishedMethodContractV1\s*,?\s*\}\s*from\s*'
        r'"\./generated/establishedMethodContractsV1"\s*;'
    )
    _unique_match(import_pattern, source, "TypeScript method generated import", flags=re.DOTALL)
    if source.count('"./generated/establishedMethodContractsV1"') != 1:
        raise ContractError("TypeScript method consumer must have exactly one generated import")

    helper = _source_section(
        source,
        "function establishedMethodRequirementsV1(",
        "function nonblank(",
        "TypeScript method generated helper",
    )
    _expect_compact_source(
        helper,
        """
function establishedMethodRequirementsV1(
  method: string,
): readonly MethodCapabilityRequirementV2[] | null {
  const contract = establishedMethodContractV1(method, method);
  if (!contract) return null;
  return freezeRequirements(contract.capability_requirements.map((item) => requirement(
    item.capability_id,
    item.cell_id,
    item.option,
  )));
}
""",
        "TypeScript method generated helper",
    )

    resolver = _source_section(
        source,
        "export function methodCapabilityRequirementsV2(",
        "function exactRegistryMatch(",
        "TypeScript method resolver",
    )
    adoption_fallback = """
  const established = establishedMethodRequirementsV1(method);
  if (established) return established;
  if (Object.hasOwn(SIMPLE_REQUIREMENTS, method)) {
    return freezeRequirements(SIMPLE_REQUIREMENTS[method as keyof typeof SIMPLE_REQUIREMENTS]);
  }
"""
    compact_resolver = _compact_source(resolver)
    compact_adoption_fallback = _compact_source(adoption_fallback)
    if compact_adoption_fallback not in compact_resolver:
        raise ContractError("TypeScript method generated lookup/fallback placement drifted")
    compact_switch = _compact_source("switch (method) {")
    if (
        compact_resolver.count(compact_switch) != 1
        or compact_resolver.index(compact_adoption_fallback) > compact_resolver.index(compact_switch)
    ):
        raise ContractError("TypeScript method generated lookup/fallback placement drifted")
    if resolver.count("establishedMethodRequirementsV1(") != 1:
        raise ContractError("TypeScript method resolver must call the generated helper exactly once")

    simple_table = _source_section(
        source,
        "const SIMPLE_REQUIREMENTS = {",
        "const GROUP_OPTION_REQUIREMENTS = {",
        "TypeScript SIMPLE_REQUIREMENTS",
    )
    target_keys = tuple(method["factory_key"] for method in methods)
    for key in target_keys:
        if re.search(rf"(?:^|\n)\s*{re.escape(key)}\s*:", simple_table):
            raise ContractError(f"TypeScript method consumer retains stale {key} simple literal")
        if re.search(rf'["\']{re.escape(key)}["\']', resolver):
            raise ContractError(f"TypeScript method consumer retains stale {key} resolver branch")
    literal_calls = _literal_call_tuples(source, "requirement")
    for key, identity in _primary_ts_tuples(methods).items():
        if identity in literal_calls:
            raise ContractError(f"TypeScript method consumer retains stale {key} primary literal {identity}")


def _verify_typescript_canonical_adoption(methods: list[dict[str, Any]], source: str) -> None:
    import_pattern = (
        r'import\s*\{\s*ESTABLISHED_METHOD_CONTRACTS_V1\s*,\s*'
        r'establishedCanonicalTableOwnerOptionsV1\s*,?\s*\}\s*from\s*'
        r'"\.\./domain/generated/establishedMethodContractsV1"\s*;'
    )
    _unique_match(import_pattern, source, "TypeScript canonical generated import", flags=re.DOTALL)
    if source.count('"../domain/generated/establishedMethodContractsV1"') != 1:
        raise ContractError("TypeScript canonical consumer must have exactly one generated import")

    helper = _source_section(
        source,
        "function establishedCanonicalTableRequirementsV1(",
        "function sortedDistinctCapabilityCells(",
        "TypeScript canonical generated helper",
    )
    _expect_compact_source(
        helper,
        """
function establishedCanonicalTableRequirementsV1(
  tableId: string,
): readonly MethodCapabilityRequirementV2[] | null {
  const ownerOptions = establishedCanonicalTableOwnerOptionsV1(tableId);
  if (ownerOptions.length === 0) return null;
  return ownerOptions.flatMap((ownerOption) => ESTABLISHED_METHOD_CONTRACTS_V1.flatMap((contract) => (
    contract.capability_requirements
      .filter((item) => item.option === ownerOption)
      .map((item) => requirement(item.capability_id, item.cell_id, item.option))
  )));
}
""",
        "TypeScript canonical generated helper",
    )

    table_resolver = _source_section(
        source,
        "export function nativeCapabilityRequirementsForTableV2(",
        "function capabilityCellsForTable(",
        "TypeScript canonical table resolver",
    )
    fallback = """
  const established = establishedCanonicalTableRequirementsV1(tableId);
  if (established) return established;
"""
    compact_table_resolver = _compact_source(table_resolver)
    compact_fallback = _compact_source(fallback)
    if compact_fallback not in compact_table_resolver:
        raise ContractError("TypeScript canonical generated lookup/fallback placement drifted")
    compact_terminal_fallback = _compact_source("return null;")
    if (
        compact_table_resolver.count(compact_terminal_fallback) != 1
        or compact_table_resolver.index(compact_fallback)
        > compact_table_resolver.index(compact_terminal_fallback)
    ):
        raise ContractError("TypeScript canonical generated lookup/fallback placement drifted")
    if table_resolver.count("establishedCanonicalTableRequirementsV1(") != 1:
        raise ContractError("TypeScript canonical table resolver must call the generated helper exactly once")
    literal_calls = _literal_call_tuples(table_resolver, "requirement")
    for method in methods:
        key = method["factory_key"]
        prefix = method["canonical_table_rules"][0]["value"]
        if prefix in table_resolver:
            raise ContractError(f"TypeScript canonical consumer retains stale {key} prefix branch")
        identity = _primary_ts_tuples(methods)[key]
        if identity in literal_calls:
            raise ContractError(f"TypeScript canonical consumer retains stale {key} primary literal {identity}")


def _verify_rust_core_adoption(source: str) -> None:
    generated_module = _source_section(
        source,
        "pub mod generated {",
        "mod methods;",
        "Rust core generated module",
    )
    _expect_compact_source(
        generated_module,
        """
pub mod generated {
    mod established_method_contracts_v1;

    pub use established_method_contracts_v1::{
        EstablishedCanonicalTableRuleV1, EstablishedCapabilityRequirementV1,
        EstablishedMethodContractV1, established_canonical_table_owner_options_v1,
        established_method_contract_v1,
    };
}
""",
        "Rust core generated module/export",
    )


def _verify_rust_cli_adoption(methods: list[dict[str, Any]], source: str) -> None:
    helper = _source_section(
        source,
        'const ESTABLISHED_CLI_REQUIREMENT_ROLE_ORDER_V1: [&str; 2] = ["primary", "base"];',
        "fn required_cli_capability_cells(",
        "Rust CLI generated helper",
    )
    _expect_compact_source(
        helper,
        """
const ESTABLISHED_CLI_REQUIREMENT_ROLE_ORDER_V1: [&str; 2] = ["primary", "base"];

fn push_generated_established_cli_capability_cells(
    required_cells: &mut Vec<RequiredCliCapabilityCellV2>,
    method: AnalysisMethod,
    config_kind: &'static str,
) -> Result<()> {
    let contract =
        qpls_core::generated::established_method_contract_v1(method.as_str(), config_kind)
            .ok_or_else(|| {
                unmapped_cli_capability_error(
                    method,
                    config_kind,
                    CliCapabilityMappingFailure::UnmappedMethodConfig,
                )
            })?;
    for role in ESTABLISHED_CLI_REQUIREMENT_ROLE_ORDER_V1 {
        for requirement in contract
            .capability_requirements
            .iter()
            .filter(|requirement| requirement.role == role)
        {
            push_required_cli_capability_cell(
                required_cells,
                requirement.capability_id,
                requirement.cell_id,
                requirement.capability_version,
            );
        }
    }
    Ok(())
}
""",
        "Rust CLI generated lookup/helper/order/filter",
    )
    if helper.count("qpls_core::generated::established_method_contract_v1(") != 1:
        raise ContractError("Rust CLI generated helper must call the exported lookup exactly once")

    resolver = _source_section(
        source,
        "fn required_cli_capability_cells(",
        "fn require_cli_capability_availability(",
        "Rust CLI capability resolver",
    )
    expected_prefix = """
fn required_cli_capability_cells(
    recipe: &AnalysisRecipe,
) -> Result<Vec<RequiredCliCapabilityCellV2>> {
    let method = recipe.settings.method;
    let Some(config) = recipe.method_config.as_ref() else {
        return Err(unmapped_cli_capability_error(
            method,
            "<missing>",
            CliCapabilityMappingFailure::MissingMethodConfig,
        ));
    };
    if !config.supports_method(method) {
        return Err(unmapped_cli_capability_error(
            method,
            config.kind(),
            CliCapabilityMappingFailure::MethodConfigMismatch,
        ));
    }

    let mut required_cells = Vec::new();
    match (method, config) {
"""
    if not _compact_source(resolver).startswith(_compact_source(expected_prefix)):
        raise ContractError("Rust CLI missing/mismatch dynamic fallback preflight drifted")
    if resolver.count('"<missing>"') != 1:
        raise ContractError("Rust CLI missing-config error bytes drifted")

    arm_patterns = {
        "cca": r"\(\s*AnalysisMethod::Cca\s*,\s*MethodConfig::Cca\s*\)",
        "gsca": r"\(\s*AnalysisMethod::Gsca\s*,\s*MethodConfig::Gsca\s*\)",
        "ipma": r"\(\s*AnalysisMethod::Ipma\s*,\s*MethodConfig::Ipma\s*\{\s*\.\.\s*\}\s*\)",
        "nca": r"\(\s*AnalysisMethod::Nca\s*,\s*MethodConfig::Nca\s*\{\s*\.\.\s*\}\s*\)",
    }
    expected_body = """
        push_generated_established_cli_capability_cells(
            &mut required_cells,
            method,
            config.kind(),
        )?;
"""
    for key in (method["factory_key"] for method in methods):
        arm = _unique_match(
            rf"{arm_patterns[key]}\s*=>\s*\{{(?P<body>.*?)\n\s*\}}",
            resolver,
            f"Rust CLI adopted {key} arm",
            flags=re.DOTALL,
        )
        _expect_compact_source(arm.group("body"), expected_body, f"Rust CLI adopted {key} arm")
        method_variant = {
            "cca": "Cca",
            "gsca": "Gsca",
            "ipma": "Ipma",
            "nca": "Nca",
        }[key]
        if len(re.findall(rf"\bAnalysisMethod::{method_variant}\b", resolver)) != 1:
            raise ContractError(f"Rust CLI adopted {key} method branch must occur exactly once")
        if len(re.findall(rf"\bMethodConfig::{method_variant}\b", resolver)) != 1:
            raise ContractError(f"Rust CLI adopted {key} config branch must occur exactly once")
    if resolver.count("push_generated_established_cli_capability_cells(") != len(methods):
        raise ContractError("Rust CLI generated helper must route exactly the four established arms")

    stale_calls = _literal_call_tuples(resolver, "push_required_cli_capability_cell")
    for key, identity in _primary_rust_tuples(methods).items():
        if identity in stale_calls:
            raise ContractError(f"Rust CLI resolver retains stale {key} primary literal {identity}")

    legacy = _unique_match(
        r"\(\s*AnalysisMethod::Legacy\s*,\s*MethodConfig::Legacy\s*\)\s*=>\s*\{(?P<body>.*?)\n\s*\}",
        resolver,
        "Rust CLI legacy dynamic fallback",
        flags=re.DOTALL,
    ).group("body")
    _expect_compact_source(
        legacy,
        """
            return Err(unmapped_cli_capability_error(
                method,
                config.kind(),
                CliCapabilityMappingFailure::UnmappedMethodConfig,
            ));
""",
        "Rust CLI legacy dynamic fallback",
    )
    wildcard = _unique_match(
        r"(?:^|\n)\s*_\s*=>\s*\{(?P<body>.*?)\n\s*\}",
        resolver,
        "Rust CLI wildcard dynamic fallback",
        flags=re.DOTALL,
    ).group("body")
    _expect_compact_source(
        wildcard,
        """
            return Err(unmapped_cli_capability_error(
                method,
                config.kind(),
                CliCapabilityMappingFailure::UnmappedMethodConfig,
            ));
""",
        "Rust CLI wildcard dynamic fallback",
    )


def _verify_consumer_adoption(methods: list[dict[str, Any]], root: Path) -> None:
    _verify_generated_contract_role_ownership(methods)
    _verify_typescript_method_adoption(
        methods, _read_utf8(root, PARITY_SOURCES["typescript_method"])
    )
    _verify_typescript_canonical_adoption(
        methods, _read_utf8(root, PARITY_SOURCES["typescript_canonical"])
    )
    _verify_rust_core_adoption(_read_utf8(root, PARITY_SOURCES["rust_core_module"]))
    _verify_rust_cli_adoption(methods, _read_utf8(root, PARITY_SOURCES["rust_cli"]))


def load_contract_model(root: Path = ROOT) -> dict[str, Any]:
    root = root.resolve(strict=True)
    if not root.is_dir():
        raise ContractError(f"repository root is not a directory: {root}")
    schema = strict_json_file(_input_file(root, SCHEMA_RELATIVE))
    _validate_schema(schema)
    contract_document = strict_json_file(_input_file(root, CONTRACT_RELATIVE))
    _validate_json_schema_instance(contract_document, schema, schema, "contract")
    contract = _validate_contract(contract_document, root)
    registry = _expect_dict(strict_json_file(_input_file(root, REGISTRY_RELATIVE)), "registry")
    registry_schema_version = _expect_integer(registry.get("registry_schema_version"), "registry.registry_schema_version")
    if registry_schema_version != 2:
        raise ContractError("established-method contract v1 requires registry schema version 2")
    capabilities = _expect_list(registry.get("capabilities"), "registry.capabilities")

    methods: list[dict[str, Any]] = []
    input_paths = {
        SCHEMA_RELATIVE,
        CONTRACT_RELATIVE,
        GENERATOR_RELATIVE,
        REGISTRY_RELATIVE,
        *PARITY_SOURCES.values(),
    }
    for method in contract["methods"]:
        manifest_identity = _manifest_identity(root, method["manifest"])
        requirements = [
            _resolve_requirement(root, registry_schema_version, capabilities, requirement)
            for requirement in method["capability_requirements"]
        ]
        primary = next(item for item in requirements if item["role"] == "primary")
        if primary["identity_manifest"] != method["manifest"]:
            raise ContractError(
                f"{method['factory_key']} primary registry cell must reference {method['manifest']}"
            )
        if (
            primary["cell_id"] != manifest_identity["feature_id"]
            or primary["capability_version"] != manifest_identity["method_version"]
        ):
            raise ContractError(f"{method['factory_key']} primary cell and manifest identity differ")
        input_paths.update(
            {
                method["manifest"],
                method["factory"]["reference_script"],
                method["factory"]["reference_report"],
                *(item["identity_manifest"] for item in requirements),
            }
        )
        methods.append(
            {
                "factory_key": method["factory_key"],
                "manifest": method["manifest"],
                "manifest_identity": manifest_identity,
                "runtime": method["runtime"],
                "capability_requirements": requirements,
                "canonical_table_rules": method["canonical_table_rules"],
                "factory": method["factory"],
            }
        )

    _verify_factory_parity(methods, root)
    _verify_consumer_adoption(methods, root)
    for relative in sorted(input_paths):
        _input_file(root, relative)
    return {
        "schema_version": 1,
        "contract_id": contract["contract_id"],
        "contract_sha256": semantic_sha256(contract),
        "registry_schema_version": registry_schema_version,
        "methods": methods,
        "input_paths": sorted(input_paths),
    }


def _assert_no_forbidden_keys(value: Any, field: str = "generated output") -> None:
    if isinstance(value, dict):
        forbidden = sorted(set(value).intersection(FORBIDDEN_OUTPUT_KEYS))
        if forbidden:
            raise ContractError(f"{field} contains forbidden authority keys: {forbidden}")
        for key, item in value.items():
            _assert_no_forbidden_keys(item, f"{field}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            _assert_no_forbidden_keys(item, f"{field}[{index}]")


def _ownership_document(model: dict[str, Any]) -> dict[str, Any]:
    document = {
        "schema_version": 1,
        "contract_id": model["contract_id"],
        "contract_sha256": model["contract_sha256"],
        "adoption_phase": 2,
        "consumer_adopted": True,
        "shadow_only": False,
        "registry": {
            "path": REGISTRY_RELATIVE,
            "registry_schema_version": model["registry_schema_version"],
        },
        "input_paths": model["input_paths"],
        "generated_targets": list(OUTPUT_RELATIVES),
        "consumer_sources": PARITY_SOURCES,
        "methods": model["methods"],
    }
    _assert_no_forbidden_keys(document)
    return document


def _json_pretty_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, allow_nan=False, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")


def _quoted(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _render_typescript(model: dict[str, Any]) -> bytes:
    lines = [
        "// @generated by validation/established_method_contract_codegen_v1.py; do not edit.",
        f"// contract-sha256: {model['contract_sha256']}",
        "",
        "export type EstablishedMethodRequirementRoleV1 = \"base\" | \"primary\";",
        "",
        "export interface EstablishedCapabilityRequirementV1 {",
        "  readonly registry_schema_version: 2;",
        "  readonly capability_id: string;",
        "  readonly cell_id: string;",
        "  readonly capability_version: string;",
        "  readonly option: string;",
        "  readonly role: EstablishedMethodRequirementRoleV1;",
        "  readonly identity_manifest: string;",
        "}",
        "",
        "export interface EstablishedCanonicalTableRuleV1 {",
        "  readonly match: \"prefix\";",
        "  readonly value: string;",
        "  readonly owner_options: readonly string[];",
        "}",
        "",
        "export interface EstablishedMethodContractV1 {",
        "  readonly factory_key: string;",
        "  readonly manifest: string;",
        "  readonly method_version: string;",
        "  readonly analysis_method: string;",
        "  readonly method_config_kind: string;",
        "  readonly workbench_kind: string;",
        "  readonly result_discriminator: string;",
        "  readonly capability_requirements: readonly EstablishedCapabilityRequirementV1[];",
        "  readonly canonical_table_rules: readonly EstablishedCanonicalTableRuleV1[];",
        "}",
        "",
        "export const ESTABLISHED_METHOD_CONTRACTS_V1 = Object.freeze([",
    ]
    for method in model["methods"]:
        runtime = method["runtime"]
        lines.extend(
            [
                "  Object.freeze({",
                f"    factory_key: {_quoted(method['factory_key'])},",
                f"    manifest: {_quoted(method['manifest'])},",
                f"    method_version: {_quoted(method['manifest_identity']['method_version'])},",
                f"    analysis_method: {_quoted(runtime['analysis_method'])},",
                f"    method_config_kind: {_quoted(runtime['method_config_kind'])},",
                f"    workbench_kind: {_quoted(runtime['workbench_kind'])},",
                f"    result_discriminator: {_quoted(runtime['result_discriminator'])},",
                "    capability_requirements: Object.freeze([",
            ]
        )
        for requirement in method["capability_requirements"]:
            lines.extend(
                [
                    "      Object.freeze({",
                    f"        registry_schema_version: {requirement['registry_schema_version']} as const,",
                    f"        capability_id: {_quoted(requirement['capability_id'])},",
                    f"        cell_id: {_quoted(requirement['cell_id'])},",
                    f"        capability_version: {_quoted(requirement['capability_version'])},",
                    f"        option: {_quoted(requirement['option'])},",
                    f"        role: {_quoted(requirement['role'])},",
                    f"        identity_manifest: {_quoted(requirement['identity_manifest'])},",
                    "      }),",
                ]
            )
        lines.extend(["    ]),", "    canonical_table_rules: Object.freeze(["])
        for rule in method["canonical_table_rules"]:
            owners = ", ".join(_quoted(owner) for owner in rule["owner_options"])
            lines.extend(
                [
                    "      Object.freeze({",
                    "        match: \"prefix\" as const,",
                    f"        value: {_quoted(rule['value'])},",
                    f"        owner_options: Object.freeze([{owners}]),",
                    "      }),",
                ]
            )
        lines.extend(["    ]),", "  }),"])
    lines.extend(
        [
            "]) satisfies readonly EstablishedMethodContractV1[];",
            "",
            "const EMPTY_OWNER_OPTIONS_V1: readonly string[] = Object.freeze([]);",
            "",
            "export function establishedMethodContractV1(",
            "  analysisMethod: string,",
            "  methodConfigKind: string,",
            "): EstablishedMethodContractV1 | null {",
            "  return ESTABLISHED_METHOD_CONTRACTS_V1.find((contract) =>",
            "    contract.analysis_method === analysisMethod && contract.method_config_kind === methodConfigKind",
            "  ) ?? null;",
            "}",
            "",
            "export function establishedCanonicalTableOwnerOptionsV1(tableId: string): readonly string[] {",
            "  for (const contract of ESTABLISHED_METHOD_CONTRACTS_V1) {",
            "    for (const rule of contract.canonical_table_rules) {",
            "      if (rule.match === \"prefix\" && tableId.startsWith(rule.value)) return rule.owner_options;",
            "    }",
            "  }",
            "  return EMPTY_OWNER_OPTIONS_V1;",
            "}",
            "",
        ]
    )
    return "\n".join(lines).encode("utf-8")


def _rust_constant(key: str, suffix: str) -> str:
    return f"{key.upper()}_{suffix}"


def _render_rust(model: dict[str, Any]) -> bytes:
    lines = [
        "// @generated by validation/established_method_contract_codegen_v1.py; do not edit.",
        f"// contract-sha256: {model['contract_sha256']}",
        "",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub struct EstablishedCapabilityRequirementV1 {",
        "    pub registry_schema_version: u32,",
        "    pub capability_id: &'static str,",
        "    pub cell_id: &'static str,",
        "    pub capability_version: &'static str,",
        "    pub option: &'static str,",
        "    pub role: &'static str,",
        "    pub identity_manifest: &'static str,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub struct EstablishedCanonicalTableRuleV1 {",
        "    pub prefix: &'static str,",
        "    pub owner_options: &'static [&'static str],",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub struct EstablishedMethodContractV1 {",
        "    pub factory_key: &'static str,",
        "    pub manifest: &'static str,",
        "    pub method_version: &'static str,",
        "    pub analysis_method: &'static str,",
        "    pub method_config_kind: &'static str,",
        "    pub workbench_kind: &'static str,",
        "    pub result_discriminator: &'static str,",
        "    pub capability_requirements: &'static [EstablishedCapabilityRequirementV1],",
        "    pub canonical_table_rules: &'static [EstablishedCanonicalTableRuleV1],",
        "}",
        "",
    ]
    for method in model["methods"]:
        key = method["factory_key"]
        requirements_constant = _rust_constant(key, "CAPABILITY_REQUIREMENTS_V1")
        owners_constant = _rust_constant(key, "OWNER_OPTIONS_V1")
        rules_constant = _rust_constant(key, "CANONICAL_TABLE_RULES_V1")
        requirements = method["capability_requirements"]
        if len(requirements) == 1:
            lines.extend(
                [
                    f"const {requirements_constant}: &[EstablishedCapabilityRequirementV1] =",
                    "    &[EstablishedCapabilityRequirementV1 {",
                ]
            )
        else:
            lines.append(f"const {requirements_constant}: &[EstablishedCapabilityRequirementV1] = &[")
        for requirement in requirements:
            if len(requirements) != 1:
                lines.append("    EstablishedCapabilityRequirementV1 {")
            lines.extend(
                [
                    f"        registry_schema_version: {requirement['registry_schema_version']},",
                    f"        capability_id: {_quoted(requirement['capability_id'])},",
                    f"        cell_id: {_quoted(requirement['cell_id'])},",
                    f"        capability_version: {_quoted(requirement['capability_version'])},",
                    f"        option: {_quoted(requirement['option'])},",
                    f"        role: {_quoted(requirement['role'])},",
                    f"        identity_manifest: {_quoted(requirement['identity_manifest'])},",
                ]
            )
            lines.append("    }];" if len(requirements) == 1 else "    },")
        if len(requirements) != 1:
            lines.append("];")
        owners = ", ".join(_quoted(item) for item in method["canonical_table_rules"][0]["owner_options"])
        lines.append(f"const {owners_constant}: &[&str] = &[{owners}];")
        lines.extend(
            [
                f"const {rules_constant}: &[EstablishedCanonicalTableRuleV1] =",
                "    &[EstablishedCanonicalTableRuleV1 {",
                f"        prefix: {_quoted(method['canonical_table_rules'][0]['value'])},",
                f"        owner_options: {owners_constant},",
                "    }];",
                "",
            ]
        )
    lines.append("pub const ESTABLISHED_METHOD_CONTRACTS_V1: &[EstablishedMethodContractV1] = &[")
    for method in model["methods"]:
        key = method["factory_key"]
        runtime = method["runtime"]
        lines.extend(
            [
                "    EstablishedMethodContractV1 {",
                f"        factory_key: {_quoted(key)},",
                f"        manifest: {_quoted(method['manifest'])},",
                f"        method_version: {_quoted(method['manifest_identity']['method_version'])},",
                f"        analysis_method: {_quoted(runtime['analysis_method'])},",
                f"        method_config_kind: {_quoted(runtime['method_config_kind'])},",
                f"        workbench_kind: {_quoted(runtime['workbench_kind'])},",
                f"        result_discriminator: {_quoted(runtime['result_discriminator'])},",
                f"        capability_requirements: {_rust_constant(key, 'CAPABILITY_REQUIREMENTS_V1')},",
                f"        canonical_table_rules: {_rust_constant(key, 'CANONICAL_TABLE_RULES_V1')},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "pub fn established_method_contract_v1(",
            "    analysis_method: &str,",
            "    method_config_kind: &str,",
            ") -> Option<&'static EstablishedMethodContractV1> {",
            "    ESTABLISHED_METHOD_CONTRACTS_V1.iter().find(|contract| {",
            "        contract.analysis_method == analysis_method",
            "            && contract.method_config_kind == method_config_kind",
            "    })",
            "}",
            "",
            "pub fn established_canonical_table_owner_options_v1(table_id: &str) -> &'static [&'static str] {",
            "    for contract in ESTABLISHED_METHOD_CONTRACTS_V1 {",
            "        for rule in contract.canonical_table_rules {",
            "            if table_id.starts_with(rule.prefix) {",
            "                return rule.owner_options;",
            "            }",
            "        }",
            "    }",
            "    &[]",
            "}",
            "",
        ]
    )
    return "\n".join(lines).encode("utf-8")


def render_outputs(root: Path = ROOT) -> dict[str, bytes]:
    model = load_contract_model(root)
    outputs = {
        OUTPUT_RELATIVES[0]: _json_pretty_bytes(_ownership_document(model)),
        OUTPUT_RELATIVES[1]: _render_typescript(model),
        OUTPUT_RELATIVES[2]: _render_rust(model),
    }
    if tuple(outputs) != OUTPUT_RELATIVES:
        raise ContractError("renderer target order differs from the declared output lock")
    for relative, data in outputs.items():
        text = data.decode("utf-8")
        for forbidden in sorted(FORBIDDEN_OUTPUT_KEYS):
            if re.search(rf"\b{re.escape(forbidden)}\b", text):
                raise ContractError(
                    f"generated output {relative} contains forbidden authority field {forbidden!r}"
                )
    return outputs


def check_outputs(root: Path = ROOT) -> list[str]:
    rendered = render_outputs(root)
    targets = dict(zip(OUTPUT_RELATIVES, declared_output_paths(root)))
    mismatches: list[str] = []
    for relative, expected in rendered.items():
        target = targets[relative]
        if not target.is_file():
            mismatches.append(f"missing: {relative}")
            continue
        try:
            actual = target.read_bytes()
        except OSError as error:
            raise ContractError(f"cannot read generated target {relative}: {error}") from error
        if actual != expected:
            mismatches.append(f"stale: {relative}")
    return mismatches


def _atomic_write(target: Path, data: bytes) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    temp_name: str | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb",
            dir=target.parent,
            prefix=f".{target.name}.",
            suffix=".tmp",
            delete=False,
        ) as handle:
            temp_name = handle.name
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_name, target)
        temp_name = None
    finally:
        if temp_name is not None:
            try:
                Path(temp_name).unlink()
            except FileNotFoundError:
                pass


def write_outputs(root: Path = ROOT) -> list[str]:
    rendered = render_outputs(root)
    targets = dict(zip(OUTPUT_RELATIVES, declared_output_paths(root)))
    changed: list[str] = []
    for relative, data in rendered.items():
        target = targets[relative]
        if target.is_file() and target.read_bytes() == data:
            continue
        _atomic_write(target, data)
        changed.append(relative)
    remaining = check_outputs(root)
    if remaining:
        raise ContractError(f"post-write verification failed: {remaining}")
    return changed


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="verify generated outputs (default)")
    mode.add_argument("--write", action="store_true", help="atomically update the three generated outputs")
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root; useful for isolated tests")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.write:
            changed = write_outputs(args.root)
            print(f"generated {len(OUTPUT_RELATIVES)} outputs; changed {len(changed)}")
            for relative in changed:
                print(relative)
            return 0
        mismatches = check_outputs(args.root)
        if mismatches:
            for mismatch in mismatches:
                print(mismatch, file=sys.stderr)
            return 1
        print(f"established method contract outputs are current ({len(OUTPUT_RELATIVES)} files)")
        return 0
    except (ContractError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
