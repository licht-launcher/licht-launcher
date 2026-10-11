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
