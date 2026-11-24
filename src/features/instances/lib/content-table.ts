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

/** Entries of the chosen kind whose title, file name or version contains the search. */
export function filterContentEntries(
  entries: readonly InstanceContentEntry[],
  filter: ContentFilter,
  search: string,
): InstanceContentEntry[] {
  const normalizedSearch = search.trim().toLocaleLowerCase();
  return entries.filter(
    (entry) =>
      (filter === "all" || entry.kind === filter) &&
      (normalizedSearch === "" ||
        [entry.title, entry.name, entry.version ?? ""].some((value) =>
          value.toLocaleLowerCase().includes(normalizedSearch),
        )),
  );
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
