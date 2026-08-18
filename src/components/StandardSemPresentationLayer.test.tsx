import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  commitStandardSemPresentationRequiredText,
  createStandardSemPresentationObject,
  moveStandardSemPresentationObject,
} from "./StandardSemPresentationLayer";

describe("StandardSemPresentationLayer", () => {
  it("creates every presentation-only object kind with explicit non-colour semantics", () => {
    const objects = (["caption", "note", "shape", "image", "line"] as const)
      .map((kind, index) => createStandardSemPresentationObject(kind, `object:${kind}`, index));
    expect(objects.map((object) => object.kind)).toEqual(["caption", "note", "shape", "image", "line"]);
    expect(objects.find((object) => object.kind === "image")).toMatchObject({
      assetRef: "asset:replace-me",
      altText: "Describe this image",
    });
    expect(objects.find((object) => object.kind === "line")).toMatchObject({ endMarker: "arrow" });
  });

  it("moves both boxed objects and line endpoints without changing their content", () => {
    const caption = createStandardSemPresentationObject("caption", "caption:1");
    const line = createStandardSemPresentationObject("line", "line:1");
    expect(moveStandardSemPresentationObject(caption, 10, -5)).toMatchObject({ id: "caption:1", text: "Caption", x: 90, y: 75 });
    expect(moveStandardSemPresentationObject(line, 10, -5)).toMatchObject({ id: "line:1", x1: 90, y1: 75, x2: 270, y2: 75 });
  });

  it("never commits an empty required image field", () => {
    const committed: string[] = [];
    expect(commitStandardSemPresentationRequiredText("   ", "asset:original", (value) => committed.push(value))).toBe(false);
    expect(committed).toEqual([]);
    expect(commitStandardSemPresentationRequiredText("  asset:replacement  ", "asset:original", (value) => committed.push(value))).toBe(true);
    expect(committed).toEqual(["asset:replacement"]);
  });

  it("exposes keyboard creation, selection, editing, movement, deletion, and text labels", () => {
    const source = readFileSync("src/components/StandardSemPresentationLayer.tsx", "utf8");
    expect(source).toContain('role="toolbar"');
    expect(source).toContain("Add presentation-only");
    expect(source).toContain('event.key === "Delete" || event.key === "Backspace"');
    expect(source).toContain("ArrowLeft");
    expect(source).toContain("Alternative text");
    expect(source).toContain("onBlur={commitDraft}");
    expect(source).toContain("the saved value is unchanged");
    expect(source).toContain("Presentation only · excluded from scientific hash");
  });
});
