import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import NativeResultsSurface, {
  ProcessConditionalPlotView,
  ProcessJohnsonNeymanPlot,
} from "./NativeResultsSurface";
import { buildNativeResultNavigation, resultTableForItem } from "./nativeResults";
import { processV2Run } from "./nativeProcessTestFixture";

describe("NativeResultsSurface PROCESS v2 plots", () => {
  it("assigns collision-free accessible IDs to simultaneous Unicode and punctuation plots", () => {
    const graph = processV2Run().result!.regression!.process!.graph_v2!;
    const firstPlot = structuredClone(graph.plots[0]);
    const secondPlot = structuredClone(graph.plots[0]);
    firstPlot.plot_id = "plot:moderation:X->Y@\u00c5";
    firstPlot.moderation_id = "moderation:X->Y@\u00c5";
    secondPlot.plot_id = "plot:moderation:X->Y@?";
    secondPlot.moderation_id = "moderation:X->Y@?";
    const firstJn = structuredClone(graph.johnson_neyman[0]);
    const secondJn = structuredClone(graph.johnson_neyman[0]);
    if (firstJn.status !== "available" || secondJn.status !== "available") throw new Error("fixture requires available JN rows");
    firstJn.moderation_id = "moderation:X->Y@\u00c5";
    secondJn.moderation_id = "moderation:X->Y@?";
    const markup = renderToStaticMarkup(<>
      <ProcessConditionalPlotView plot={firstPlot} outcome="Y" />
      <ProcessConditionalPlotView plot={secondPlot} outcome="Y" />
      <ProcessJohnsonNeymanPlot plot={firstJn} />
      <ProcessJohnsonNeymanPlot plot={secondJn} />
    </>);
    const labelledBy = [...markup.matchAll(/aria-labelledby="([^"]+)"/g)].map((match) => match[1]);
    const ids = [...markup.matchAll(/<(?:title|desc) id="([^"]+)"/g)].map((match) => match[1]);
    expect(labelledBy).toHaveLength(4);
    expect(new Set(labelledBy).size).toBe(4);
    expect(ids).toHaveLength(8);
    expect(new Set(ids).size).toBe(8);
    expect(labelledBy.every((pair) => pair.split(" ").every((id) => ids.includes(id)))).toBe(true);
  });

  it("renders only persisted conditional points alongside the table-backed simple slopes", () => {
    const run = processV2Run();
    const navigation = buildNativeResultNavigation(run);
    const selectedItem = navigation.groups.flatMap((group) => group.items)
      .find((item) => item.id === "process_simple_slopes")!;
    const markup = renderToStaticMarkup(<NativeResultsSurface
      runs={[run]}
      selectedRun={run}
      selectedRunId={run.id}
      setSelectedRunId={() => undefined}
      navigation={navigation}
      selectedItem={selectedItem}
      selectedTable={resultTableForItem(navigation, selectedItem.id)}
      setSelectedTableId={() => undefined}
      propertiesOpen
    />);
    expect(markup).toContain("data-process-plot-id=\"plot:moderation:X-&gt;Y@W\"");
    expect(markup).toContain("Engine-persisted conditional outcome data");
    expect(markup).toContain("original-sample raw moderator probes");
    expect(markup).toContain('aria-label="Series legend for X × W → Y"');
    expect(markup).toContain("circle markers");
    expect(markup).toContain("square markers");
    expect(markup).toContain("triangle markers");
    expect(markup).toContain('stroke-dasharray="8 3"');
    expect(markup).toContain("Exact predicted values and confidence intervals are available in the adjacent conditional outcome plot data table");
    expect(navigation.groups.flatMap((group) => group.items).map((item) => item.id))
      .toContain("process_conditional_plot_points");
    expect(markup).toContain("data-result-table-id=\"process_simple_slopes\"");
  });

  it("gives all nine continuous-by-continuous series distinct non-color style signatures", () => {
    const base = processV2Run().result!.regression!.process!.graph_v2!.plots[0];
    const plot = structuredClone(base);
    plot.series = Array.from({ length: 9 }, (_, index) => ({
      ...structuredClone(base.series[index % base.series.length]),
      series_id: `series:${index}:test`,
    }));
    const markup = renderToStaticMarkup(<ProcessConditionalPlotView plot={plot} outcome="Y" />);
    const lineStyles = [...markup.matchAll(/data-process-series-style="([^"]+)"/g)].map((match) => match[1]);
    const legendStyles = [...markup.matchAll(/data-process-legend-style="([^"]+)"/g)].map((match) => match[1]);
    expect(lineStyles).toHaveLength(9);
    expect(new Set(lineStyles).size).toBe(9);
    expect(legendStyles).toEqual(lineStyles);
    expect(markup).toContain("circle markers");
    expect(markup).toContain("square markers");
    expect(markup).toContain("triangle markers");
  });

  it("renders the persisted 101-point Johnson-Neyman curve without UI inference", () => {
    const run = processV2Run();
    const navigation = buildNativeResultNavigation(run);
    const selectedItem = navigation.groups.flatMap((group) => group.items)
      .find((item) => item.id === "process_johnson_neyman")!;
    const markup = renderToStaticMarkup(<NativeResultsSurface
      runs={[run]}
      selectedRun={run}
      selectedRunId={run.id}
      setSelectedRunId={() => undefined}
      navigation={navigation}
      selectedItem={selectedItem}
      selectedTable={resultTableForItem(navigation, selectedItem.id)}
      setSelectedTableId={() => undefined}
      propertiesOpen={false}
    />);
    expect(markup).toContain("data-process-jn-moderation=\"moderation:X-&gt;Y@W\"");
    expect(markup).toContain("All 101 curve points, intervals, roots, and regions were persisted by the engine");
    expect(markup).toContain("Exact effect, SE, and confidence bounds are available in the adjacent Johnson");
    expect(navigation.groups.flatMap((group) => group.items).map((item) => item.id))
      .toContain("process_johnson_neyman_curve_points");
    expect(markup).toContain("data-result-table-id=\"process_johnson_neyman\"");
  });
});
