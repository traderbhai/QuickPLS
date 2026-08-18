import type { Edge } from "@xyflow/react";
import {
  convertNativeCovarianceToPresentationV4,
  convertNativeCovarianceToScientificV4,
  markNativeCovarianceRoleConversionV4,
  withoutNativeCovarianceAuthoringV4,
} from "../domain/semModelV4Authoring";
import type { PathEdgeData, SemModelV4AuthoringEndpoint } from "../types";

export type NativePathRole = "structural" | "control" | "covariance";

function dataFor(edge: Edge): PathEdgeData & Record<string, unknown> {
  return { ...((edge.data as PathEdgeData | undefined) ?? {}) };
}

function textLabel(edge: Edge): string {
  return typeof edge.label === "string" ? edge.label : "";
}

export function nativePathDisplayLabel(edge: Edge, role: NativePathRole): string {
  const data = dataFor(edge);
  if (role === "control") return data.controlLabel?.trim() || textLabel(edge);
  return textLabel(edge);
}

export function nativePathLabelPatch(edge: Edge, role: NativePathRole, label: string): Partial<Edge> {
  if (role !== "control") return { label };
  const data = dataFor(edge);
  const controlLabel = label.trim() || null;
  return {
    label: controlLabel ?? "Control",
    data: { ...withoutNativeCovarianceAuthoringV4(data), role: "control", controlLabel },
  };
}

export function nativePathRolePatch(edge: Edge, role: NativePathRole): Partial<Edge> {
  const previousRole = (edge.data as PathEdgeData | undefined)?.role;
  const previousData = dataFor(edge);
  const previousLabel = textLabel(edge).trim();
  if (role === "structural") {
    const structuralData = withoutNativeCovarianceAuthoringV4(previousData);
    delete structuralData.role;
    delete structuralData.controlLabel;
    const wasGeneratedLabel = previousLabel === "Control" || previousLabel === "Covariance";
    return { data: structuralData, label: wasGeneratedLabel ? "Path" : previousLabel || "Path" };
  }
  if (role === "covariance") {
    delete previousData.controlLabel;
    const covariance = previousRole === "covariance"
      ? { ...edge, data: { ...previousData, role: "covariance" as const } }
      : markNativeCovarianceRoleConversionV4({ ...edge, data: { ...previousData, role: "covariance" } });
    return {
      data: covariance.data,
      label: !previousRole || previousLabel === "Path" || previousLabel === "Control"
        ? "Covariance"
        : previousLabel || "Covariance",
    };
  }
  const retainedCustomLabel = previousData.controlLabel?.trim()
    || (!["", "Path", "Control", "Covariance"].includes(previousLabel) ? previousLabel : null);
  const controlData = withoutNativeCovarianceAuthoringV4(previousData);
  return {
    data: { ...controlData, role: "control", controlLabel: retainedCustomLabel },
    label: retainedCustomLabel ?? "Control",
  };
}

export function nativeCovarianceScientificPatch(
  edge: Edge,
  endpoints: { left: SemModelV4AuthoringEndpoint | null; right: SemModelV4AuthoringEndpoint | null } = { left: null, right: null },
): Partial<Edge> {
  const converted = convertNativeCovarianceToScientificV4(edge, endpoints);
  return { data: converted.data };
}

export function nativeCovariancePresentationPatch(edge: Edge): Partial<Edge> {
  const converted = convertNativeCovarianceToPresentationV4(edge);
  return { data: converted.data };
}
