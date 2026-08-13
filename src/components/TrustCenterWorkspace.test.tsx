import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useWorkspace } from "../store";
import { TrustCenterWorkspace } from "./TrustCenterWorkspace";

describe("TrustCenterWorkspace method truth", () => {
  beforeEach(() => {
    useWorkspace.getState().resetProject();
    useWorkspace.getState().setAnalysisSettings({ method: "permutation", permutationSamples: 999 });
    vi.stubGlobal("window", {});
  });

  it("renders Structural Path Randomization as experimental candidate scope", () => {
    const html = renderToStaticMarkup(<TrustCenterWorkspace />);
    vi.unstubAllGlobals();

    expect(html).toContain("Freedman-Lane permutation");
    expect(html).toContain("experimental");
    expect(html).toContain("Scope transparency");
    expect(html).toContain("docs/methods/PERMUTATION_ENGINE_V1.md");
    expect(html).not.toContain("Freedman-Lane permutation</td><td>PLS-SEM</td><td>Validated scope");
  });
});
