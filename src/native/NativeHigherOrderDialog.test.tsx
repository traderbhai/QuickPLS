import type { Node } from "@xyflow/react";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { ConstructData } from "../types";
import NativeHigherOrderDialog from "./NativeHigherOrderDialog";

const ordinaryNodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["x1"] } },
  { id: "z", position: { x: 0, y: 180 }, data: { label: "Reputation", shortName: "REP", mode: "reflective", indicators: ["z1"] } },
  { id: "f", position: { x: 0, y: 360 }, data: { label: "Formative block", shortName: "FORM", mode: "formative", indicators: ["f1"] } },
  { id: "y", position: { x: 360, y: 90 }, data: { label: "Outcome", shortName: "OUT", mode: "reflective", indicators: ["y1"] } },
];

const applied = async () => ({ status: "applied" as const, constructId: "derived:hoc" });

describe("NativeHigherOrderDialog", () => {
  it("uses compact conceptual controls, derives RR, and hides approach choice under Advanced", () => {
    const html = renderToStaticMarkup(<NativeHigherOrderDialog
      nodes={ordinaryNodes}
      edges={[]}
      request={{ kind: "create", selectedComponentIds: ["x", "z"] }}
      commit={vi.fn(applied)}
      close={vi.fn()}
    />);

    expect(html).toContain("Conceptual direction");
    expect(html).toContain("HOC explains its dimensions");
    expect(html).toContain("Dimensions form the HOC");
    expect(html).toContain("<dt>Type</dt><dd>RR</dd>");
    expect(html).toContain("Disjoint two-stage (Recommended)");
    expect(html).toContain("<summary>Advanced</summary>");
    expect(html).toContain("Construction approach");
    expect(html).toContain("Formative block [FORM]");
    expect(html).toContain("Mode B · Choose Mode B dimensions in a separate HOC.");
    expect(html).not.toContain("Short name");
    expect(html).not.toContain("<dt>Execution</dt>");
    expect(html).toContain(">Create</button>");
    expect(html.match(/type="checkbox" checked=""/g)).toHaveLength(2);
  });

  it("retains a mixed preselection so the user can correct it locally", () => {
    const html = renderToStaticMarkup(<NativeHigherOrderDialog
      nodes={ordinaryNodes}
      edges={[]}
      request={{ kind: "create", selectedComponentIds: ["x", "f"] }}
      commit={vi.fn(applied)}
      close={vi.fn()}
    />);

    expect(html.match(/type="checkbox" checked=""/g)).toHaveLength(2);
    expect(html).toContain("<dt>Type</dt><dd>Choose dimensions</dd>");
  });

  it("shows the bounded initial path only for calculation-ready creation", () => {
    const html = renderToStaticMarkup(<NativeHigherOrderDialog
      nodes={ordinaryNodes}
      edges={[]}
      request={{ kind: "create", selectedComponentIds: ["x", "z"], requireInitialPath: true }}
      commit={vi.fn(applied)}
      close={vi.fn()}
    />);

    expect(html).toContain("Initial model path");
    expect(html).toContain("HOC → construct");
    expect(html).toContain("Outcome [OUT]");
  });

  it("opens an editable strict HOC by output identity while retaining its term identity", () => {
    const hoc: Node<ConstructData> = {
      id: "derived:hoc",
      position: { x: 220, y: 90 },
      data: {
        label: "Corporate standing",
        shortName: "STAND",
        mode: "reflective",
        indicators: [],
        semantic: "higher_order",
        higherOrder: {
          id: "term:hoc",
          components: ["x", "z"],
          method: "two_stage",
          canonicalApproach: "embedded_two_stage",
          measurementType: "reflective_reflective",
          stage_one_recipe: null,
        },
      },
    };
    const html = renderToStaticMarkup(<NativeHigherOrderDialog
      nodes={[...ordinaryNodes, hoc]}
      edges={[
        { id: "x-y", source: "x", target: "y" },
        { id: "hoc-y", source: hoc.id, target: "y" },
      ]}
      request={{ kind: "edit", constructId: hoc.id }}
      commit={vi.fn(applied)}
      close={vi.fn()}
    />);

    expect(html).toContain('value="Corporate standing"');
    expect(html).toContain("<dt>Approach</dt><dd>Embedded two-stage");
    expect(html).toContain(">Save</button>");
    expect(html).not.toContain("Initial model path");
    expect(html).not.toContain("Short name");
    expect(html.match(/type="checkbox" checked=""/g)).toHaveLength(2);
  });

  it("keeps a legacy hybrid HOC readable but not editable", () => {
    const hybrid: Node<ConstructData> = {
      id: "derived:hybrid",
      position: { x: 220, y: 90 },
      data: {
        label: "Legacy standing",
        shortName: "LEG",
        mode: "reflective",
        indicators: [],
        semantic: "higher_order",
        higherOrder: {
          id: "term:hybrid",
          components: ["x", "z"],
          method: "hybrid",
          canonicalApproach: "hybrid",
          measurementType: "reflective_reflective",
          stage_one_recipe: null,
        },
      },
    };
    const html = renderToStaticMarkup(<NativeHigherOrderDialog
      nodes={[...ordinaryNodes, hybrid]}
      edges={[]}
      request={{ kind: "edit", constructId: "term:hybrid" }}
      commit={vi.fn(applied)}
      close={vi.fn()}
    />);

    expect(html).toContain("Legacy standing");
    expect(html).toContain("<dt>Approach</dt><dd>Hybrid</dd>");
    expect(html).toContain("compatibility-only");
    expect(html).toContain(">Close</button>");
    expect(html).not.toContain(">Save</button>");
  });

  it("keeps the dialog open and busy until the async authority result is applied", () => {
    const source = readFileSync("src/native/NativeHigherOrderDialog.tsx", "utf8");

    expect(source).toContain("Promise<NativeHigherOrderDialogCommitResult>");
    expect(source).toContain("const result = await (request.kind");
    expect(source).toContain('if (result.status === "applied")');
    expect(source).toContain("aria-busy={commitPending}");
    expect(source).toContain("disabled={commitPending}");
    expect(source).toContain("onPendingChange?.(true)");
    expect(source).toContain("onPendingChange?.(false)");
    expect(source).toContain('status: "blocked" | "cancelled" | "stale" | "rejected"');
    expect(source).toContain("data-commit-status={commitIssue.status}");
    expect(source).toContain("higherOrderCommitMessage(commitIssue)");
  });

  it("removes generic edit and delete commands from the HOC context menu", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    const start = source.indexOf("const contextConstructIsHigherOrder");
    const end = source.indexOf("const showWorkspaceContextMenu", start);
    const contextMenuSource = source.slice(start, end);

    expect(start).toBeGreaterThan(0);
    expect(end).toBeGreaterThan(start);
    expect(contextMenuSource).toContain('command.id !== "edit-selection" && command.id !== "delete-selection"');
    expect(contextMenuSource).toContain('command.id === "edit-higher-order"');
    expect(contextMenuSource).toContain("disabled: contextHigherOrderAuthorityCommand ? !contextHigherOrderAuthorityCommand.enabled : true");
    expect(contextMenuSource).toContain('id: "edit-higher-order-construct"');
    expect(contextMenuSource).toContain('id: "remove-higher-order-construct"');
  });

  it("awaits HOC gateway and Save As Revision outcomes before reporting application", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    const start = source.indexOf("const launchHigherOrderRevision = async");
    const end = source.indexOf("const removeHigherOrderConstruct", start);
    const commitSource = source.slice(start, end);

    expect(start).toBeGreaterThan(0);
    expect(end).toBeGreaterThan(start);
    expect(commitSource).toContain("const result = await useInternalProjectArchiveV6Session.getState()");
    expect(commitSource).toContain("const result = await commitGatewayDesktopCommand");
    expect(commitSource).toContain('if (result === "saved")');
    expect(commitSource).toContain('return { status: "cancelled"');
    expect(commitSource).toContain('return { status: "stale"');
    expect(commitSource).toContain('return { status: "rejected"');
    expect(source).toContain("? !higherOrderCommitPending");
    expect(source).toContain("onPendingChange={setHigherOrderCommitPending}");
  });
});
