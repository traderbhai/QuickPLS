import { describe, expect, it } from "vitest";
import { moderationAnchorDropProjectionV1 } from "./ModelCanvas";

describe("ModelCanvas moderation-anchor drag projection", () => {
  it("snaps an off-path drop to the bounded focal polyline position", () => {
    const projection = moderationAnchorDropProjectionV1(
      [{ x: 0, y: 0 }, { x: 100, y: 0 }, { x: 100, y: 100 }],
      { x: 189, y: 39 },
    );

    expect(projection.fraction).toBeCloseTo(0.75);
    expect(projection.position).toEqual({ x: 89, y: 39 });
  });

  it("clamps persisted anchor fractions to the archive-safe range", () => {
    expect(moderationAnchorDropProjectionV1([{ x: 0, y: 0 }, { x: 100, y: 0 }], { x: -61, y: -11 })).toEqual({
      fraction: 0.2,
      position: { x: 9, y: -11 },
    });
    expect(moderationAnchorDropProjectionV1([{ x: 0, y: 0 }, { x: 100, y: 0 }], { x: 139, y: -11 })).toEqual({
      fraction: 0.8,
      position: { x: 69, y: -11 },
    });
  });
});
