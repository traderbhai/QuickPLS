import { describe, expect, it } from "vitest";
import { NATIVE_MICOM_SETUP_COMMAND } from "./ProductivityOverlays";

describe("ProductivityOverlays MICOM command", () => {
  it("offers the exact standalone MICOM preset without legacy combined wording", () => {
    expect(NATIVE_MICOM_SETUP_COMMAND).toEqual({
      label: "Setup MICOM v3.1",
      detail: "Prepare standalone MICOM and review its requirements",
    });
    expect(`${NATIVE_MICOM_SETUP_COMMAND.label} ${NATIVE_MICOM_SETUP_COMMAND.detail}`)
      .not.toContain("MICOM + MGA");
  });
});
