import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

export type ContentFilter = "all" | InstanceContentKind;
export type ContentSortKey = "name";
export type ContentSortDir = "asc" | "desc";

export function formatContentTitle(name: string): string {
  const withoutExtension = name.replace(/\.[^.]+$/, "");
  return withoutExtension
    .replace(/[-_.]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

export function sortContentEntries(
  entries: InstanceContentEntry[],
  dir: ContentSortDir,
): InstanceContentEntry[] {
  const sign = dir === "asc" ? 1 : -1;
  return [...entries].sort(
    (left, right) =>
      left.title.localeCompare(right.title, undefined, {
        sensitivity: "base",
      }) * sign,
  );
}

export function toggleContentSort(
  currentKey: ContentSortKey,
  currentDir: ContentSortDir,
  nextKey: ContentSortKey,
): { key: ContentSortKey; dir: ContentSortDir } {
  if (currentKey === nextKey) {
    return { key: currentKey, dir: currentDir === "asc" ? "desc" : "asc" };
  }
  return { key: nextKey, dir: "asc" };
}
