import type { Edge, Node } from "@xyflow/react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { ConstructData } from "../types";
import { newNativeScientificCovarianceEdgeV4, withNativeConstructEstimandV4 } from "../domain/semModelV4Authoring";
import {
  NativeSemConstructAuthoringFields,
  NativeSemCovarianceAuthoringFields,
} from "./NativeSemScientificAuthoring";
import { NativeModelInspector } from "./NativeModelInspector";

function node(id: string, indicators: string[] = [`${id}1`, `${id}2`]): Node<ConstructData> {
  return {
    id,
    type: "construct",
    position: { x: 0, y: 0 },
    data: { label: id.toUpperCase(), shortName: id.toUpperCase(), mode: "reflective", indicators },
  };
}

describe("SemModelV4 scientific authoring UI", () => {
  it("renders an accessible factor-versus-composite decision with an actionable legacy diagnostic", () => {
    const html = renderToStaticMarkup(<NativeSemConstructAuthoringFields node={node("x")} onCommit={vi.fn()} />);
    expect(html).toContain("Scientific representation");
    expect(html).toContain("Model authority");
    expect(html).toContain("Choose representation");
    expect(html).toContain("Composite");
    expect(html).toContain("Common factor");
    expect(html).toContain("still needs a factor-versus-composite decision");
    expect(html).toContain("aria-describedby=");
    expect(html).toContain("role=\"status\"");
  });

  it("shows the persisted marker selector for a confirmed common factor", () => {
    const factor = withNativeConstructEstimandV4(node("x", ["x2", "x1"]), {
      kind: "common_factor",
      marker_indicator: "x2",
    });
    const html = renderToStaticMarkup(<NativeSemConstructAuthoringFields node={factor} onCommit={vi.fn()} />);
    expect(html).toContain("Marker indicator");
    expect(html).toContain('<option value="x2" selected="">x2</option>');
    expect(html).toContain("Common factor confirmed with x2 as marker");
    expect(html).toContain("Clear representation decision");
  });

  it("offers exactly the four covariance uses and endpoint controls for residual covariance", () => {
    const nodes = [node("x"), node("y")];
    const base = newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y");
    const residual: Edge = {
      ...base,
      data: {
        role: "covariance",
        semModelV4: {
          version: 1,
          covariance: {
            kind: "scientific",
            origin: "explicit_conversion",
            left: { kind: "residual_of", id: "observed:x1" },
            right: { kind: "residual_of", id: "observed:y2" },
          },
        },
      },
    };
    const html = renderToStaticMarkup(<NativeSemCovarianceAuthoringFields edge={residual} nodes={nodes} edges={[residual]} onCommit={vi.fn()} />);
    expect(html).toContain("Model covariance");
    expect(html).toContain("Residual/error covariance");
    expect(html).toContain("Disturbance covariance");
    expect(html).toContain("Presentation only");
    expect(html).toContain("Residual covariance endpoints");
    expect(html).toContain("X indicator");
    expect(html).toContain("Y indicator");
    expect(html).toContain("Apply relationship use");
  });

  it("explains why an invalid disturbance covariance cannot be applied", () => {
    const nodes = [node("x"), node("y")];
    const edge: Edge = {
      ...newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y"),
      data: {
        role: "covariance",
        semModelV4: {
          version: 1,
          covariance: {
            kind: "scientific",
            origin: "explicit_conversion",
            left: { kind: "disturbance_of", id: "construct:x" },
            right: { kind: "disturbance_of", id: "construct:y" },
          },
        },
      },
    };
    const html = renderToStaticMarkup(<NativeSemCovarianceAuthoringFields edge={edge} nodes={nodes} edges={[edge]} onCommit={vi.fn()} />);
    expect(html).toContain("No incoming path");
    expect(html).toContain("has no structural disturbance to correlate");
    expect(html).toContain("Use Model covariance or add an identified incoming structural path");
    expect(html).toContain("disabled=\"\"");
  });

  it("exposes Standard representation controls while keeping unqualified covariance authoring Labs-scoped", () => {
    const nodes = [node("x"), node("y")];
    const covariance = newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y");
    const standardInspector = renderToStaticMarkup(<NativeModelInspector
      initialMode="expert"
      initialTab="parameter"
      nodesOverride={nodes}
      edgesOverride={[covariance]}
      selectedNodeIdOverride="x"
      selectedEdgeIdOverride={null}
      experimentalLabsEnabledOverride={false}
    />);
    const standardCovariance = renderToStaticMarkup(<NativeModelInspector
      initialMode="expert"
      initialTab="parameter"
      nodesOverride={nodes}
      edgesOverride={[covariance]}
      selectedNodeIdOverride={null}
      selectedEdgeIdOverride={covariance.id}
      experimentalLabsEnabledOverride={false}
    />);
    const labsCovariance = renderToStaticMarkup(<NativeModelInspector
      initialMode="expert"
      initialTab="parameter"
      nodesOverride={nodes}
      edgesOverride={[covariance]}
      selectedNodeIdOverride={null}
      selectedEdgeIdOverride={covariance.id}
      experimentalLabsEnabledOverride
    />);

    expect(standardInspector).toContain("Scientific representation");
    expect(standardInspector).toContain("Common factor");
    expect(standardCovariance).not.toContain("Residual/error covariance");
    expect(labsCovariance).toContain("Model covariance");
    expect(labsCovariance).toContain("Residual/error covariance");
  });
});
