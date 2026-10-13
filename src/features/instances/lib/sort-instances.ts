import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export type InstanceSort = "lastPlayed" | "name";

function openedAtMs(value: string | null): number | null {
  if (value === null) {
    return null;
  }
  const ms = Date.parse(value);
  return Number.isNaN(ms) ? null : ms;
}

export function sortInstances(
  instances: InstanceEntry[],
  sort: InstanceSort,
): InstanceEntry[] {
  const sorted = [...instances];
  if (sort === "name") {
    sorted.sort((left, right) => left.name.localeCompare(right.name));
    return sorted;
  }

  sorted.sort((left, right) => {
    const leftOpened = openedAtMs(left.lastOpenedAt);
    const rightOpened = openedAtMs(right.lastOpenedAt);
    if (leftOpened !== null && rightOpened !== null) {
      if (leftOpened !== rightOpened) {
        return rightOpened - leftOpened;
      }
      return left.name.localeCompare(right.name);
    }
    if (leftOpened !== null) {
      return -1;
    }
    if (rightOpened !== null) {
      return 1;
    }
    return left.name.localeCompare(right.name);
  });
  return sorted;
}
