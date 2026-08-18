use qpls_core::{CAPABILITY_REGISTRY_V2_JSON, CapabilityRegistryV2, CapabilityRegistryV2Error};
use serde::{Deserialize, Serialize};

const CAPABILITY_REGISTRY_COMMAND_RESPONSE_SCHEMA_VERSION: u32 = 1;

/// Read-only bridge for inspecting the exact Capability Registry V2 source
/// embedded in the desktop executable. Execution and Calculate visibility do
/// not consult this command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapabilityRegistryCommandResponseV2 {
    response_schema_version: u32,
    registry_schema_version: u32,
    registry_id: String,
    registry_version: String,
    source_sha256: String,
    capability_row_count: usize,
    active_row_count: usize,
    option_cell_count: usize,
    source_json: String,
}

fn embedded_capability_registry_response_v2()
-> Result<CapabilityRegistryCommandResponseV2, CapabilityRegistryV2Error> {
    let registry = CapabilityRegistryV2::embedded()?;
    let capability_row_count = registry.capabilities.len();
    let active_row_count = registry.catalogue_snapshot.active_row_count;
    let option_cell_count = registry.option_cells().count();
    Ok(CapabilityRegistryCommandResponseV2 {
        response_schema_version: CAPABILITY_REGISTRY_COMMAND_RESPONSE_SCHEMA_VERSION,
        registry_schema_version: registry.registry_schema_version,
        registry_id: registry.registry_id,
        registry_version: registry.registry_version,
        source_sha256: registry.source_sha256,
        capability_row_count,
        active_row_count,
        option_cell_count,
        source_json: CAPABILITY_REGISTRY_V2_JSON.to_owned(),
    })
}

#[tauri::command]
pub(crate) fn capability_registry_v2() -> Result<CapabilityRegistryCommandResponseV2, String> {
    embedded_capability_registry_response_v2().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use sha2::{Digest, Sha256};

    #[test]
    fn command_returns_the_exact_validated_registry_source_and_digest() {
        let response = capability_registry_v2().expect("embedded registry command response");
        let source: Value = serde_json::from_str(&response.source_json).expect("registry JSON");
        let rows = source["capabilities"].as_array().expect("capability rows");
        let option_cell_count = rows
            .iter()
            .map(|row| {
                row["option_cells"]
                    .as_array()
                    .expect("option-cell array")
                    .len()
            })
            .sum::<usize>();

        assert_eq!(response.response_schema_version, 1);
        assert_eq!(response.registry_schema_version, 2);
        assert_eq!(response.registry_id, "quickpls.capability_registry.v2");
        assert_eq!(response.registry_version, source["registry_version"]);
        assert_eq!(response.capability_row_count, rows.len());
        assert_eq!(response.active_row_count, 43);
        assert_eq!(response.option_cell_count, option_cell_count);
        assert_eq!(
            response.source_sha256,
            format!("{:x}", Sha256::digest(response.source_json.as_bytes()))
        );
    }

    #[test]
    fn response_wire_is_strict_and_contains_no_execution_switch() {
        let response = capability_registry_v2().expect("embedded registry command response");
        let mut wire = serde_json::to_value(response).expect("response wire");
        assert!(wire.get("standard_available").is_none());
        assert!(wire.get("experimental_labs_enabled").is_none());

        wire["standard_available"] = Value::Bool(true);
        assert!(
            serde_json::from_value::<CapabilityRegistryCommandResponseV2>(wire).is_err(),
            "unknown execution or visibility switches must fail closed"
        );
    }
}
