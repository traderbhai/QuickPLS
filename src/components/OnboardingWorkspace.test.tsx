import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useWorkspace } from "../store";
import { OnboardingWorkspace } from "./OnboardingWorkspace";

describe("OnboardingWorkspace sample gallery", () => {
  beforeEach(() => {
    useWorkspace.getState().resetProject();
    vi.stubGlobal("window", { dispatchEvent: vi.fn() });
  });

  it("advertises only the four complete bundled sample workflows", () => {
    const html = renderToStaticMarkup(<OnboardingWorkspace />);

    expect(html).toContain("Corporate reputation");
    expect(html).toContain("Full 344-case model with 8 constructs, 31 modeled indicators, 13 paths");
    expect(html).toContain("Simple reflective PLS-SEM");
    expect(html).toContain("Mediation");
    expect(html).toContain("Organizational Identification Model");
    expect(html).toContain("305 cases, 4 reflective constructs, 21 modeled indicators, 3 paths");
    expect(html).toContain("four complete bundled workflows");
    expect(html).not.toContain("MICOM / MGA");
    expect(html).not.toContain("CB-SEM CFA");
    expect(html).not.toContain("PLSpredict");
  });

  it("keeps the selected sample identity in state for the native open command", () => {
    useWorkspace.getState().setOnboardingState({ selectedDemo: "mediation" });
    const html = renderToStaticMarkup(<OnboardingWorkspace />);

    expect(html).toContain("sample-project-card active");
    expect(useWorkspace.getState().onboardingState.selectedDemo).toBe("mediation");
  });
});
