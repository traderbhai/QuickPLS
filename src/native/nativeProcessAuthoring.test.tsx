import { renderToStaticMarkup } from "react-dom/server";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, Dataset, RunMonitorState } from "../types";
import { capabilityAvailabilityV2 } from "../domain/capabilitySurfaceV2";
import { capabilityRegistryV2 } from "../domain/capabilityRegistryV2";
import type { MethodCapabilityRegistryReaderV2 } from "../domain/methodCapabilityRegistryV2";
import NativeCalculationDialog from "./NativeCalculationDialog";

const settings: AnalysisUiSettings = {
  method: "regression",
  weightingScheme: "path",
  preprocessing: "unstandardized",
  bootstrapSamples: 10_000,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20260812,
  workers: 4,
  confidenceLevel: 0.95,
  regressionType: "process",
  regressionOutcome: "Y",
  regressionPredictors: "X,M1,M2,M3,M4,W,B",
  regressionControls: "C",
  regressionBootstrap: true,
  robustSe: "hc3",
  processGraph: {
    model: "graph",
    focal_predictor: "X",
    paths: [
      { from: "X", to: "Y" },
      { from: "X", to: "M1" },
      { from: "M1", to: "M2" },
      { from: "M2", to: "Y" },
      { from: "X", to: "M3" },
      { from: "M3", to: "Y" },
      { from: "X", to: "M4" },
      { from: "M4", to: "Y" },
    ],
    moderators: [
      { variable: "W", scale: "continuous" },
      { variable: "B", scale: "binary_0_1" },
    ],
    moderations: [
      { from: "X", to: "Y", moderator: "W", conditioning_moderator: "B" },
      { from: "X", to: "M3", moderator: "W" },
      { from: "M4", to: "Y", moderator: "B" },
    ],
    continuous_product_centering: "equation_complete_case_mean_v1",
  },
};

const columns = ["X", "M1", "M2", "M3", "M4", "W", "B", "C", "D", "Y"];
const dataset: Dataset = {
  id: "process-authoring-data",
  name: "process-authoring.csv",
  columns,
  rows: Array.from({ length: 40 }, (_, index) => ({
    X: index / 5,
    M1: index / 7 + index % 3,
    M2: index / 9 + index % 4,
    M3: index / 11 + index % 5,
    M4: index / 13 + index % 6,
    W: index % 9 - 4,
    B: index % 2,
    C: index % 7 / 3,
    D: index % 5 / 2,
    Y: index / 2 + index % 8,
  })),
  rowCount: 40,
  missing: 0,
  fingerprint: "sha256:process-authoring",
  kind: "raw",
  columnMetadata: columns.map((name) => ({
    name,
    label: null,
    column_type: "numeric",
    role: "unassigned",
    scale_type: "continuous",
    missing_markers: [],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: {},
  })),
};

const runMonitor: RunMonitorState = {
  status: "idle",
  phase: "",
  message: "",
  completedUnits: 0,
  totalUnits: 0,
  startedAt: null,
  completedAt: null,
  activeJobId: null,
  lastRunId: null,
  error: null,
  logs: [],
};

const processLabsRegistry: MethodCapabilityRegistryReaderV2 = {
  quickPlsCell(cellId) {
    return capabilityRegistryV2.quickPlsCell(cellId).map((match) => {
      if (cellId !== "qpls3.standalone.process") return match;
      return {
        ...match,
        cell: {
          ...match.cell,
          coverage_state: "partial",
          evidence_state: "engine_only",
          surface: "labs",
        },
      };
    });
  },
  availability(capabilityId, cellId, experimentalLabsEnabled) {
    const match = this.quickPlsCell(cellId).find((candidate) => candidate.row.capability_id === capabilityId);
    if (!match) throw new Error(`Missing PROCESS test capability cell ${capabilityId}::${cellId}`);
    return capabilityAvailabilityV2(match.cell, experimentalLabsEnabled);
  },
};

describe("native PROCESS v2 graph authoring", () => {
  it("uses stable ordered row identities so controlled select edits retain focus", () => {
    const source = readFileSync("src/native/NativeProcessSetup.tsx", "utf8");
    expect(source).toContain('key={`process-path-${index}`}');
    expect(source).toContain('key={`process-moderator-${index}`}');
    expect(source).toContain('key={`process-moderation-${index}`}');
    expect(source).not.toContain('key={`${index}-${pathToken(path)}`}');
    expect(source).not.toContain('key={`${index}-${moderator.variable}`}');
    expect(source).not.toContain('key={`${index}-${pathToken(moderation)}`}');
  });

  it("renders explicit paths, reusable moderator coding, first/second stage and mixed three-way controls", () => {
    const markup = renderToStaticMarkup(<NativeCalculationDialog
      kind="regression"
      experimentalLabsEnabled
      capabilityRegistry={processLabsRegistry}
      setKind={() => undefined}
      settings={settings}
      setSettings={() => undefined}
      readiness={{ canRun: true, summary: "Ready", blockers: [], warnings: [], items: [] }}
      runMonitor={runMonitor}
      dataset={dataset}
      analysisColumns={columns}
      nodes={[]}
      edges={[]}
      start={() => undefined}
      cancel={() => undefined}
      close={() => undefined}
    />);
    expect(markup).toContain("Graph-defined Path Analysis / PROCESS");
    expect(markup).toContain("id=\"nd-calculation-process-graph\"");
    expect(markup).toContain("id=\"nd-process-outcome\"");
    expect(markup).toContain("id=\"nd-process-focal\"");
    expect(markup.match(/data-process-path-row/g)).toHaveLength(8);
    expect(markup.match(/data-process-moderator-row/g)).toHaveLength(2);
    expect(markup.match(/data-process-moderation-row/g)).toHaveLength(3);
    expect(markup).toContain("Binary (must be exact 0/1; uncentered)");
    expect(markup).not.toContain("original sample raw mean - SD, mean, and mean + SD");
    expect(markup).not.toMatch(/[\u00c2\u00c3\ufffd]|\u00e2[\u2020\u20ac]/u);
    expect(markup).toContain("Start graph-defined path analysis with bootstrap");
    expect(markup).toContain("id=\"nd-process-graph-preview\"");
    expect(markup).toContain("7/8 graph predictors");
    expect(markup).toMatch(/data-process-control="true" disabled=""[^>]*><span>D<\/span>/);
    expect(markup).toContain("40 global listwise-complete cases");
    expect(markup).not.toContain("Experimental scope");
    expect(markup).not.toContain("<span>Supported setup</span>");
    expect(markup).not.toMatch(/PROCESS model [0-9]+/i);
  });
});
