import type { ModpackListItem } from "@/lib/generated/ModpackListItem";
import type { ModpackSearchPage } from "@/lib/generated/ModpackSearchPage";

/** Stable identity of a result across sources (ids only repeat across sources). */
export function modpackKey(modpack: ModpackListItem): string {
  return `${modpack.source}:${modpack.id}`;
}

/**
 * Loaded pages as one list. A pack can show up on two pages when the source's
 * ranking shifts between requests; only its first appearance is kept.
 */
export function mergeSearchPages(
  pages: readonly ModpackSearchPage[],
): ModpackListItem[] {
  const seen = new Set<string>();
  const items: ModpackListItem[] = [];
  for (const page of pages) {
    for (const item of page.items) {
      const key = modpackKey(item);
      if (!seen.has(key)) {
        seen.add(key);
        items.push(item);
      }
    }
  }
  return items;
}
