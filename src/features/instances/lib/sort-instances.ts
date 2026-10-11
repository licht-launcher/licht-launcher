import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export type InstanceSort = "lastPlayed" | "name";

export function sortInstances(
  instances: InstanceEntry[],
  sort: InstanceSort,
  lastPlayedFolder: string | null,
): InstanceEntry[] {
  const sorted = [...instances];
  if (sort === "name") {
    sorted.sort((left, right) => left.name.localeCompare(right.name));
    return sorted;
  }
  sorted.sort((left, right) => {
    if (left.folder === lastPlayedFolder) {
      return -1;
    }
    if (right.folder === lastPlayedFolder) {
      return 1;
    }
    return left.name.localeCompare(right.name);
  });
  return sorted;
}
