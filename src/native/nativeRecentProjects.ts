export interface NativeRecentProject {
  name: string;
  path: string;
  openedAt: string;
}

export const NATIVE_RECENT_PROJECTS_KEY = "quickpls.native.recent-projects.v1";
const MAX_RECENT_PROJECTS = 8;

export function loadNativeRecentProjects(storage: Pick<Storage, "getItem">): NativeRecentProject[] {
  try {
    const raw = storage.getItem(NATIVE_RECENT_PROJECTS_KEY);
    if (!raw) return [];
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return [];
    return value.filter((entry): entry is NativeRecentProject => {
      if (!entry || typeof entry !== "object") return false;
      const candidate = entry as Partial<NativeRecentProject>;
      return typeof candidate.name === "string"
        && candidate.name.trim().length > 0
        && typeof candidate.path === "string"
        && candidate.path.trim().length > 0
        && typeof candidate.openedAt === "string";
    }).slice(0, MAX_RECENT_PROJECTS);
  } catch {
    return [];
  }
}

export function rememberNativeRecentProject(
  entries: readonly NativeRecentProject[],
  project: NativeRecentProject,
): NativeRecentProject[] {
  const normalizedPath = project.path.trim();
  if (!normalizedPath) return [...entries];
  return [
    { ...project, name: project.name.trim() || "Untitled project", path: normalizedPath },
    ...entries.filter((entry) => entry.path.toLocaleLowerCase() !== normalizedPath.toLocaleLowerCase()),
  ].slice(0, MAX_RECENT_PROJECTS);
}

export function storeNativeRecentProjects(
  storage: Pick<Storage, "setItem">,
  entries: readonly NativeRecentProject[],
) {
  try {
    storage.setItem(NATIVE_RECENT_PROJECTS_KEY, JSON.stringify(entries));
  } catch {
    // localStorage can be disabled or full; recent entries are best-effort only.
  }
}
