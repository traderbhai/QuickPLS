import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { BUNDLED_SAMPLE_PROJECTS, parseNativeSampleProjectId } from "../domain/bundledSampleCatalog";
import { useWorkspace } from "../store";
import { OnboardingWorkspace } from "./OnboardingWorkspace";

describe("OnboardingWorkspace sample gallery", () => {
  beforeEach(() => {
    useWorkspace.getState().resetProject();
    vi.stubGlobal("window", { dispatchEvent: vi.fn() });
  });

  it("advertises every complete manifest-backed bundled sample workflow", () => {
    const html = renderToStaticMarkup(<OnboardingWorkspace />);

    for (const sample of BUNDLED_SAMPLE_PROJECTS) {
      expect(html).toContain(sample.label);
      expect(html).toContain(sample.detail);
    }
    expect(html).toContain(`${BUNDLED_SAMPLE_PROJECTS.length} complete bundled workflows`);
    expect(html).not.toContain("MICOM / MGA");
    expect(html).not.toContain("CB-SEM CFA");
    expect(html).not.toContain("PLSpredict");
  });

  it("keeps the selected sample identity in state for the native open command", () => {
    const mediationSampleId = parseNativeSampleProjectId("mediation");
    expect(mediationSampleId).not.toBeNull();
    useWorkspace.getState().setOnboardingState({ selectedDemo: mediationSampleId! });
    const html = renderToStaticMarkup(<OnboardingWorkspace />);

    expect(html).toContain("sample-project-card active");
    expect(useWorkspace.getState().onboardingState.selectedDemo).toBe("mediation");
  });
});
