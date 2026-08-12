import type { Edge } from "@xyflow/react";
import type { PathEdgeData } from "../types";

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
    data: { ...data, role: "control", controlLabel },
  };
}

export function nativePathRolePatch(edge: Edge, role: NativePathRole): Partial<Edge> {
  const previousRole = (edge.data as PathEdgeData | undefined)?.role;
  const previousData = dataFor(edge);
  const previousLabel = textLabel(edge).trim();
  if (role === "structural") {
    delete previousData.role;
    delete previousData.controlLabel;
    const wasGeneratedLabel = previousLabel === "Control" || previousLabel === "Covariance";
    return { data: previousData, label: wasGeneratedLabel ? "Path" : previousLabel || "Path" };
  }
  if (role === "covariance") {
    delete previousData.controlLabel;
    return {
      data: { ...previousData, role: "covariance" },
      label: !previousRole || previousLabel === "Path" || previousLabel === "Control"
        ? "Covariance"
        : previousLabel || "Covariance",
    };
  }
  const retainedCustomLabel = previousData.controlLabel?.trim()
    || (!["", "Path", "Control", "Covariance"].includes(previousLabel) ? previousLabel : null);
  return {
    data: { ...previousData, role: "control", controlLabel: retainedCustomLabel },
    label: retainedCustomLabel ?? "Control",
  };
}
