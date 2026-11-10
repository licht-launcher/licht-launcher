import type { ModpackSource } from "@/lib/generated/ModpackSource";

export type DiscoverSearch = {
  q?: string;
  page?: number;
};

/** Reads `?q=&page=` from the URL, dropping anything that is not usable. */
export function parseDiscoverSearch(
  search: Record<string, unknown>,
): DiscoverSearch {
  const q = typeof search.q === "string" ? search.q.trim() : "";
  const page = Number(search.page);
  return {
    q: q === "" ? undefined : q,
    page: Number.isInteger(page) && page > 1 ? page : undefined,
  };
}

const SOURCES: readonly ModpackSource[] = ["modrinth", "curseforge"];

export function parseModpackSource(value: string): ModpackSource | null {
  return SOURCES.find((source) => source === value) ?? null;
}

export const SOURCE_LABELS: Record<ModpackSource, string> = {
  modrinth: "Modrinth",
  curseforge: "CurseForge",
};
