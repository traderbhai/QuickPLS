use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MethodStatus {
    Experimental,
    Validated,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodCapability {
    pub id: &'static str,
    pub family: &'static str,
    pub name: &'static str,
    pub status: MethodStatus,
}

pub const METHOD_CAPABILITIES: &[MethodCapability] = &[
    MethodCapability {
        id: "pls_pm",
        family: "PLS-SEM",
        name: "PLS path modeling",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "bootstrap",
        family: "PLS-SEM",
        name: "Bootstrapping",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "pls_mediation",
        family: "PLS-SEM",
        name: "Mediation effect decomposition",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "pls_two_stage_moderation",
        family: "PLS-SEM",
        name: "Two-stage moderation",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "plsc",
        family: "PLS-SEM",
        name: "Consistent PLS",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "wpls",
        family: "PLS-SEM",
        name: "Weighted PLS",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "cca",
        family: "PLS-SEM",
        name: "Confirmatory composite analysis",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "cta_pls",
        family: "PLS-SEM",
        name: "Confirmatory tetrad analysis",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "endogeneity",
        family: "PLS-SEM",
        name: "Gaussian-copula endogeneity analysis",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "nonlinear_effects",
        family: "PLS-SEM",
        name: "Nonlinear effects",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "moderated_mediation",
        family: "PLS-SEM",
        name: "Moderated mediation",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "predict",
        family: "Prediction",
        name: "PLSpredict holdout / repeated k-fold",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "mga",
        family: "Groups",
        name: "MICOM / permutation MGA",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "ipma",
        family: "Prediction",
        name: "IPMA / cIPMA",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "cbsem",
        family: "CB-SEM",
        name: "CFA / ML SEM",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "pca",
        family: "Components",
        name: "Principal component analysis",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "gsca",
        family: "Component models",
        name: "GSCA",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "regression",
        family: "Regression",
        name: "OLS / logistic / PROCESS",
        status: MethodStatus::Validated,
    },
    MethodCapability {
        id: "nca",
        family: "Necessary conditions",
        name: "NCA",
        status: MethodStatus::Validated,
    },
];

pub fn method_status(id: &str) -> MethodStatus {
    METHOD_CAPABILITIES
        .iter()
        .find(|item| item.id == id)
        .map_or(MethodStatus::Unsupported, |item| item.status)
}
