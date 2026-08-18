use qpls_core::SemModelV4;

/// Internal-only bridge to the native SemModelV4 validation and scientific identity authority.
#[tauri::command]
pub(crate) fn internal_sem_model_v4_scientific_sha256(model: SemModelV4) -> Result<String, String> {
    model.scientific_sha256().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec,
        convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn valid_model() -> SemModelV4 {
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::nil(),
                name: "Scientific digest fixture".into(),
                constructs: vec![Construct {
                    id: "factor".into(),
                    name: "Factor".into(),
                    short_name: "F".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into(), "x3".into()],
                }],
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap()
    }

    #[test]
    fn returns_the_validated_native_scientific_digest() {
        let model = valid_model();
        let expected = model.scientific_sha256().unwrap();

        assert_eq!(
            internal_sem_model_v4_scientific_sha256(model).unwrap(),
            expected
        );
    }

    #[test]
    fn rejects_an_invalid_sem_model() {
        let mut model = valid_model();
        model.id.clear();

        assert!(internal_sem_model_v4_scientific_sha256(model).is_err());
    }
}
