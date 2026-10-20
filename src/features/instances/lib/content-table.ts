import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

export type ContentFilter = "all" | InstanceContentKind;
export type ContentSortKey = "name" | "size";
export type ContentSortDir = "asc" | "desc";

export function formatSize(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${Math.round(size / 1024)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

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
  key: ContentSortKey,
  dir: ContentSortDir,
): InstanceContentEntry[] {
  const sign = dir === "asc" ? 1 : -1;
  return [...entries].sort((left, right) => {
    const compare =
      key === "size"
        ? left.size - right.size
        : left.title.localeCompare(right.title, undefined, {
            sensitivity: "base",
          });
    return compare * sign;
  });
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
