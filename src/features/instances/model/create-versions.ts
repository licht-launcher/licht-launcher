import type { VersionSummary } from "@/lib/generated/VersionSummary";

export function createVersionChoices(
  versions: VersionSummary[],
  includeSnapshots: boolean,
): VersionSummary[] {
  return versions.filter(
    (version) =>
      version.versionType === "release" ||
      (includeSnapshots && version.versionType === "snapshot"),
  );
}

/** Minecraft family key: `1.20.1` → `1.20`; weeklies → `Snapshots`. */
export function versionMajorGroup(versionId: string): string {
  const match = /^(\d+\.\d+)/.exec(versionId);
  return match?.[1] ?? "Snapshots";
}

export type VersionMajorGroup = {
  major: string;
  versions: VersionSummary[];
};

/** Groups choices by major, preserving manifest order (newest first). */
export function groupVersionsByMajor(
  versions: VersionSummary[],
): VersionMajorGroup[] {
  const groups: VersionMajorGroup[] = [];
  const indexByMajor = new Map<string, number>();

  for (const version of versions) {
    const major = versionMajorGroup(version.id);
    const existing = indexByMajor.get(major);
    if (existing === undefined) {
      indexByMajor.set(major, groups.length);
      groups.push({ major, versions: [version] });
      continue;
    }
    groups[existing]?.versions.push(version);
  }

  return groups;
}

/** Official Java Edition update titles for known major families. */
const UPDATE_NAMES: Record<string, string> = {
  "26.3": "Wilderness Bound",
  "26.2": "Chaos Cubed",
  "26.1": "Tiny Takeover",
  "1.21": "Tricky Trials",
  "1.20": "Trails & Tales",
  "1.19": "The Wild Update",
  "1.18": "Caves & Cliffs",
  "1.17": "Caves & Cliffs",
  "1.16": "Nether Update",
  "1.15": "Buzzy Bees",
  "1.14": "Village & Pillage",
  "1.13": "Update Aquatic",
  "1.12": "World of Color",
  "1.11": "Exploration Update",
  "1.10": "Frostburn Update",
  "1.9": "Combat Update",
  "1.8": "Bountiful Update",
  "1.7": "The Update that Changed the World",
  "1.6": "Horse Update",
  "1.5": "Redstone Update",
  "1.4": "Pretty Scary Update",
  "1.3": "The Update that Changed Minecraft",
  "1.2": "The Next Adventure",
  "1.1": "The Adventure Update",
  "1.0": "Adventure Update",
};

export function updateNameForMajor(major: string): string | null {
  if (major === "Snapshots") {
    return null;
  }
  return UPDATE_NAMES[major] ?? null;
}

/** Keeps groups that have at least one version id matching the query. */
export function filterVersionGroups(
  groups: VersionMajorGroup[],
  query: string,
): VersionMajorGroup[] {
  const trimmed = query.trim().toLowerCase();
  if (trimmed === "") {
    return groups;
  }

  return groups
    .map((group) => ({
      major: group.major,
      versions: group.versions.filter((version) =>
        version.id.toLowerCase().includes(trimmed),
      ),
    }))
    .filter((group) => group.versions.length > 0);
}
