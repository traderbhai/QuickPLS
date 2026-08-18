import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import NativeHigherOrderDialog from "./NativeHigherOrderDialog";

describe("NativeHigherOrderDialog", () => {
  it("renders the bounded disjoint two-stage contract and eligible component checklist", () => {
    const html = renderToStaticMarkup(<NativeHigherOrderDialog
      nodes={[
        { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["x1"] } },
        { id: "z", position: { x: 0, y: 180 }, data: { label: "Reputation", shortName: "REP", mode: "reflective", indicators: ["z1"] } },
        { id: "f", position: { x: 0, y: 360 }, data: { label: "Formative block", shortName: "FORM", mode: "formative", indicators: ["f1"] } },
      ]}
      edges={[]}
      selectedComponentIds={["x", "z"]}
      create={vi.fn(() => ({ status: "created" as const, constructId: "hoc" }))}
      close={vi.fn()}
    />);
    expect(html).toContain("Reflective–reflective disjoint two-stage");
    expect(html).toContain("Use component scores as generated HOC indicators");
    expect(html).toContain("one HOC-to-outcome relationship");
    expect(html).toContain("no other structural path");
    expect(html).toContain("HOC bootstrapping and permutation inference remain unavailable");
    expect(html).toContain("Capability");
    expect(html).toContain("Formative block");
    expect(html).toContain("Only reflective lower-order components are supported.");
    expect(html).toContain("Create higher-order construct");
    expect(html.match(/type="checkbox" checked=""/g)).toHaveLength(2);
  });
});
