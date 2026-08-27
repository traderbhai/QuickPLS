import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import {
  prepareCanonicalResultExportV2,
  verifyPreparedCanonicalResultExportV2,
  type CanonicalResultExportFormatV2,
} from "../domain/canonicalResultCrossFormatExportV2";
import { publishNativeCanonicalResultExportAtV2 } from "../services/canonicalResultExportPublicationV2Service";

export interface PackagedQualificationExportReceiptV1 {
  readonly format: CanonicalResultExportFormatV2;
  readonly path: string;
  readonly semanticSha256: string;
  readonly exactSemanticReadback: true;
  readonly digestReadback: true;
  readonly renderedSurfaceReadback: true;
  readonly publication: unknown;
}

export interface MultiModPackagedQualificationBridgeV1 {
  readonly schemaVersion: 1;
  readonly bridgeId: "qpls.v256.multimod.packaged-qualification-bridge.v1";
  exportCanonicalMatrix(
    document: CanonicalResultDocumentV2,
    outputDirectory: string,
    fileStem: string,
  ): Promise<readonly PackagedQualificationExportReceiptV1[]>;
}

declare global {
  interface Window {
    __QPLS_MULTIMOD_PACKAGED_QUALIFICATION_V1__?: MultiModPackagedQualificationBridgeV1;
  }
}

function joinedPath(directory: string, leaf: string): string {
  const base = directory.replace(/[\\/]+$/u, "");
  if (!/^(?:[a-z]:\\|\\\\)/iu.test(base)) {
    throw new Error("Packaged qualification exports require an absolute Windows directory.");
  }
  if (!/^[a-z0-9][a-z0-9._-]*$/u.test(leaf)) {
    throw new Error("Packaged qualification export file identities must be stable.");
  }
  return `${base}\\${leaf}`;
}

async function exportCanonicalMatrix(
  document: CanonicalResultDocumentV2,
  outputDirectory: string,
  fileStem: string,
): Promise<readonly PackagedQualificationExportReceiptV1[]> {
  if (!/^[a-z0-9][a-z0-9._-]*$/u.test(fileStem)) {
    throw new Error("Packaged qualification export stem must be a stable identity.");
  }
  const matrix: Array<{
    format: CanonicalResultExportFormatV2;
    chartIds?: readonly string[];
  }> = [
    { format: "csv" },
    { format: "xlsx" },
    { format: "json" },
    { format: "html" },
    { format: "pdf" },
  ];
  const chartId = document.charts[0]?.id;
  if (chartId) {
    matrix.push({ format: "svg", chartIds: [chartId] });
    matrix.push({ format: "png", chartIds: [chartId] });
  }
  const receipts: PackagedQualificationExportReceiptV1[] = [];
  const verifiedSvgChartIds = new Set<string>();
  for (const cell of matrix) {
    const prepared = prepareCanonicalResultExportV2(document, {
      format: cell.format,
      ...(cell.chartIds ? { chartIds: cell.chartIds } : {}),
    });
    if (!prepared.ok) {
      const exactSvgFallbackVerified =
        cell.format === "png" &&
        prepared.code === "unsupported_visible_text" &&
        cell.chartIds?.every((id) => verifiedSvgChartIds.has(id)) === true;
      if (exactSvgFallbackVerified) continue;
      throw new Error(
        `Canonical ${cell.format} preparation failed: ${prepared.errors.join(" | ")}`,
      );
    }
    const readback = verifyPreparedCanonicalResultExportV2(
      document,
      prepared.artifact,
    );
    if (
      !readback.passed ||
      !readback.exact_semantic_match ||
      !readback.digest_match ||
      !readback.rendered_surface_match
    ) {
      throw new Error(
        `Canonical ${cell.format} semantic readback failed: ${readback.errors.join(" | ")}`,
      );
    }
    const destination = joinedPath(
      outputDirectory,
      `${fileStem}.${cell.format}`,
    );
    const publication = await publishNativeCanonicalResultExportAtV2(
      prepared.artifact,
      destination,
    );
    if (!publication) {
      throw new Error(`Canonical ${cell.format} publication was unexpectedly cancelled.`);
    }
    receipts.push({
      format: cell.format,
      path: destination,
      semanticSha256: prepared.artifact.semantic.semantic_sha256,
      exactSemanticReadback: true,
      digestReadback: true,
      renderedSurfaceReadback: true,
      publication,
    });
    if (cell.format === "svg") {
      for (const id of cell.chartIds ?? []) verifiedSvgChartIds.add(id);
    }
  }
  return receipts;
}

/** Installs no global in preview/release builds without the frozen build flag. */
export function installMultiModPackagedQualificationBridgeV1(): void {
  const enabled = import.meta.env.VITE_QPLS_MULTIMOD_QUALIFICATION_HARNESS_V1 === "1";
  if (!enabled) return;
  const bridge: MultiModPackagedQualificationBridgeV1 = Object.freeze({
    schemaVersion: 1,
    bridgeId: "qpls.v256.multimod.packaged-qualification-bridge.v1",
    exportCanonicalMatrix,
  });
  Object.defineProperty(window, "__QPLS_MULTIMOD_PACKAGED_QUALIFICATION_V1__", {
    configurable: false,
    enumerable: false,
    writable: false,
    value: bridge,
  });
}
