import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("ConstructNode higher-order presentation", () => {
  it("keeps HOC nodes to their label and compact marker without exposing implementation text", () => {
    const source = readFileSync("src/components/ConstructNode.tsx", "utf8");

    expect(source).toContain('data.semantic === "higher_order" ? "HOC"');
    expect(source).toContain('const higherOrder = data.semantic === "higher_order";');
    expect(source).toContain('{!higherOrder ? <div className={`construct-score');
    expect(source).toContain('{!higherOrder ? <span>[{data.shortName}]</span> : null}');
    expect(source).not.toContain('data.higherOrder.method.replaceAll');
  });
});
