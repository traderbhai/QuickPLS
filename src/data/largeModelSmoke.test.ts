import { describe, expect, it } from "vitest";
import { largeModelSmokeProject, type LargeModelSmokeProfile } from "./largeModelSmoke";

function expectProfile(profile: LargeModelSmokeProfile, constructs: number, indicators: number) {
  const first = largeModelSmokeProject(profile);
  const reopened = largeModelSmokeProject(profile);

  expect(first.nodes).toHaveLength(constructs);
  expect(first.dataset.columns).toHaveLength(indicators);
  expect(first.edges).toHaveLength(constructs - 1);
  expect(first.nodes.flatMap((node) => node.data.indicators)).toEqual(first.dataset.columns);
  expect(new Set(first.nodes.flatMap((node) => node.data.indicators)).size).toBe(indicators);
  expect(reopened).toEqual(first);

  first.nodes[0]!.position.x += 50;
  expect(largeModelSmokeProject(profile)).toEqual(reopened);
}

describe("deterministic large-model desktop fixtures", () => {
  it("recreates the applied 20-construct/80-indicator fixture exactly", () => {
    expectProfile("applied", 20, 80);
  });

  it("recreates the stress 100-construct/300-indicator fixture exactly", () => {
    expectProfile("stress", 100, 300);
  });
});
