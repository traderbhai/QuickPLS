import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import NativeHigherOrderDialog from "./NativeHigherOrderDialog";

describe("NativeHigherOrderDialog", () => {
  it("renders the bounded approach/type controls and eligible component checklist", () => {
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
    expect(html).toContain("Repeated indicators");
    expect(html).toContain("Extended repeated indicators");
    expect(html).toContain("Embedded two-stage");
    expect(html).toContain("Disjoint two-stage");
    expect(html).toContain("Reflective–reflective (RR)");
    expect(html).toContain("Mode A loadings");
    expect(html).toContain("Component loadings");
    expect(html).toContain("After creation, connect the HOC to one or more ordinary constructs.");
    expect(html).toContain("Capability");
    expect(html).toContain("Formative block");
    expect(html).toContain("This HCM type requires reflective (Mode A) lower-order components.");
    expect(html).toContain("Create higher-order construct");
    expect(html.match(/type="checkbox" checked=""/g)).toHaveLength(2);
  });

  it("requires an initial structural path when creating an immutable General SEM revision", () => {
    const html = renderToStaticMarkup(<NativeHigherOrderDialog
      nodes={[
        { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["x1"] } },
        { id: "z", position: { x: 0, y: 180 }, data: { label: "Reputation", shortName: "REP", mode: "reflective", indicators: ["z1"] } },
        { id: "y", position: { x: 360, y: 90 }, data: { label: "Outcome", shortName: "OUT", mode: "reflective", indicators: ["y1"] } },
      ]}
      edges={[]}
      selectedComponentIds={["x", "z"]}
      requireInitialPath
      create={vi.fn(() => ({ status: "created" as const, constructId: "hoc" }))}
      close={vi.fn()}
    />);
    expect(html).toContain("Initial structural path for the saved revision");
    expect(html).toContain("HOC → construct");
    expect(html).toContain("Outcome [OUT]");
  });
});
