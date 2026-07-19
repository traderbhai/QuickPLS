import type { PlsBootstrapRun } from "../types";

export interface ParsedParameterIdentity {
  kind: string;
  parts: string[];
}

export function parseParameterIdentity(identity: string): ParsedParameterIdentity | null {
  try {
    const parsed = JSON.parse(identity) as unknown;
    if (!Array.isArray(parsed) || parsed.length !== 2 || typeof parsed[0] !== "string" || !Array.isArray(parsed[1])) {
      return null;
    }
    const parts = parsed[1];
    if (!parts.every((part): part is string => typeof part === "string")) return null;
    return { kind: parsed[0], parts };
  } catch {
    return null;
  }
}

export function formatParameterIdentity(identity: string) {
  const parsed = parseParameterIdentity(identity);
  if (!parsed) return identity;
  return `${parsed.kind.replaceAll("_", " ")} | ${parsed.parts.join(" -> ")}`;
}

export function findBootstrapParameter(
  bootstrap: PlsBootstrapRun | undefined,
  kind: string,
  parts: string[],
) {
  return bootstrap?.percentile.parameters.find((parameter) => {
    const identity = parseParameterIdentity(parameter.parameter);
    return identity?.kind === kind && sameParts(identity.parts, parts);
  });
}

export function findBcaParameter(bootstrap: PlsBootstrapRun | undefined, parameterIdentity: string) {
  return bootstrap?.bca?.parameters.find((parameter) => parameter.parameter === parameterIdentity);
}

export function findStudentizedParameter(bootstrap: PlsBootstrapRun | undefined, parameterIdentity: string) {
  return bootstrap?.studentized?.parameters.find((parameter) => parameter.parameter === parameterIdentity);
}

function sameParts(left: string[], right: string[]) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}
