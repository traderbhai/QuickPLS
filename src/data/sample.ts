import type { Edge, Node } from "@xyflow/react";
import type { ConstructData, Dataset, MethodDefinition } from "../types";

export const sampleDataset: Dataset = {
  id: "corporate-reputation",
  name: "Corporate Reputation.csv",
  columns: ["COMP1", "COMP2", "COMP3", "LIKE1", "LIKE2", "CUSA1", "CUSA2", "CUSL1", "CUSL2"],
  rows: [
    { COMP1: 6, COMP2: 6, COMP3: 5, LIKE1: 6, LIKE2: 5, CUSA1: 6, CUSA2: 6, CUSL1: 5, CUSL2: 6 },
    { COMP1: 5, COMP2: 4, COMP3: 5, LIKE1: 5, LIKE2: 4, CUSA1: 5, CUSA2: 4, CUSL1: 4, CUSL2: 5 },
    { COMP1: 4, COMP2: 5, COMP3: 4, LIKE1: 4, LIKE2: 5, CUSA1: 5, CUSA2: 5, CUSL1: 5, CUSL2: 4 },
    { COMP1: 7, COMP2: 6, COMP3: 7, LIKE1: 7, LIKE2: 6, CUSA1: 7, CUSA2: 6, CUSL1: 6, CUSL2: 7 },
    { COMP1: 3, COMP2: 4, COMP3: 3, LIKE1: 4, LIKE2: 3, CUSA1: 3, CUSA2: 4, CUSL1: 3, CUSL2: 3 },
  ],
  missing: 0,
};

export const initialNodes: Array<Node<ConstructData>> = [
  { id: "competence", type: "construct", position: { x: 120, y: 125 }, data: { label: "Competence", shortName: "COMP", mode: "reflective", indicators: ["COMP1", "COMP2", "COMP3"] } },
  { id: "likeability", type: "construct", position: { x: 120, y: 335 }, data: { label: "Likeability", shortName: "LIKE", mode: "reflective", indicators: ["LIKE1", "LIKE2"] } },
  { id: "satisfaction", type: "construct", position: { x: 390, y: 230 }, data: { label: "Customer Satisfaction", shortName: "CUSA", mode: "reflective", indicators: ["CUSA1", "CUSA2"] } },
  { id: "loyalty", type: "construct", position: { x: 650, y: 230 }, data: { label: "Customer Loyalty", shortName: "CUSL", mode: "reflective", indicators: ["CUSL1", "CUSL2"] } },
];

export const initialEdges: Edge[] = [
  { id: "comp-cusa", source: "competence", target: "satisfaction", label: "Path", type: "smoothstep" },
  { id: "like-cusa", source: "likeability", target: "satisfaction", label: "Path", type: "smoothstep" },
  { id: "comp-cusl", source: "competence", target: "loyalty", label: "Path", type: "smoothstep" },
  { id: "like-cusl", source: "likeability", target: "loyalty", label: "Path", type: "smoothstep" },
  { id: "cusa-cusl", source: "satisfaction", target: "loyalty", label: "Path", type: "smoothstep" },
];

export const methods: MethodDefinition[] = [
  { id: "pls_pm", family: "PLS-SEM", name: "PLS path modeling core", status: "validated" },
  { id: "bootstrap", family: "PLS-SEM", name: "Bootstrapping", status: "validated" },
  { id: "permutation", family: "PLS-SEM", name: "Freedman-Lane permutation", status: "validated" },
  { id: "plsc", family: "PLS-SEM", name: "Consistent PLS", status: "validated" },
  { id: "wpls", family: "PLS-SEM", name: "Weighted PLS", status: "validated" },
  { id: "cca", family: "PLS-SEM", name: "Confirmatory composite analysis", status: "validated" },
  { id: "cta_pls", family: "PLS-SEM", name: "Confirmatory tetrad analysis", status: "validated" },
  { id: "endogeneity", family: "PLS-SEM", name: "Gaussian-copula endogeneity analysis", status: "validated" },
  { id: "nonlinear_effects", family: "PLS-SEM", name: "Nonlinear effects", status: "validated" },
  { id: "moderated_mediation", family: "PLS-SEM", name: "Moderated mediation", status: "validated" },
  { id: "predict", family: "Prediction", name: "PLSpredict holdout / repeated k-fold", status: "validated" },
  { id: "mga", family: "Groups", name: "MICOM / permutation MGA", status: "validated" },
  { id: "ipma", family: "Prediction", name: "IPMA / cIPMA", status: "validated" },
  { id: "cbsem", family: "CB-SEM", name: "CFA / ML SEM", status: "validated" },
  { id: "pca", family: "Components", name: "Principal component analysis", status: "validated" },
  { id: "gsca", family: "Component models", name: "GSCA", status: "validated" },
  { id: "regression", family: "Regression", name: "OLS / logistic / PROCESS", status: "validated" },
  { id: "nca", family: "Necessary conditions", name: "NCA", status: "validated" },
];
