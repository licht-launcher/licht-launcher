import {
  CATEGORY_OPTIONS,
  LOADER_OPTIONS,
  SORT_OPTIONS,
} from "@/features/discover/lib/filter-options";
import type { ModpackCategory } from "@/lib/generated/ModpackCategory";
import type { ModpackLoader } from "@/lib/generated/ModpackLoader";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";
import type { ModpackSort } from "@/lib/generated/ModpackSort";
import type { ModpackSource } from "@/lib/generated/ModpackSource";

/** No narrowing: every source, sorted by downloads. */
export const DEFAULT_SEARCH_FILTERS: ModpackSearchFilters = {
  gameVersion: null,
  loader: null,
  category: null,
  source: null,
  sort: "downloads",
};

/** Discover's URL state; defaults are left out so links stay short. */
export type DiscoverSearch = {
  q?: string;
  page?: number;
  version?: string;
  loader?: ModpackLoader;
  category?: ModpackCategory;
  source?: ModpackSource;
  sort?: ModpackSort;
};

export const MODPACK_SOURCES: readonly ModpackSource[] = [
  "modrinth",
  "curseforge",
];

function pick<T extends string>(
  value: unknown,
  allowed: readonly T[],
): T | undefined {
  return allowed.find((option) => option === value);
}

function values<T extends string>(options: readonly { value: T }[]): T[] {
  return options.map((option) => option.value);
}

/** Reads Discover's URL state, dropping anything that is not usable. */
export function parseDiscoverSearch(
  search: Record<string, unknown>,
): DiscoverSearch {
  const q = typeof search.q === "string" ? search.q.trim() : "";
  const page = Number(search.page);
  const version =
    typeof search.version === "string" ? search.version.trim() : "";
  const sort = pick(search.sort, values(SORT_OPTIONS));
  return {
    q: q === "" ? undefined : q,
    page: Number.isInteger(page) && page > 1 ? page : undefined,
    version: version === "" ? undefined : version,
    loader: pick(search.loader, values(LOADER_OPTIONS)),
    category: pick(search.category, values(CATEGORY_OPTIONS)),
    source: pick(search.source, MODPACK_SOURCES),
    sort: sort === DEFAULT_SEARCH_FILTERS.sort ? undefined : sort,
  };
}

/** The search filters a Discover URL asks for. */
export function searchFilters(search: DiscoverSearch): ModpackSearchFilters {
  return {
    gameVersion: search.version ?? null,
    loader: search.loader ?? null,
    category: search.category ?? null,
    source: search.source ?? null,
    sort: search.sort ?? DEFAULT_SEARCH_FILTERS.sort,
  };
}

/** URL params for `filters`; defaults become `undefined` and drop out. */
export function filterParams(
  filters: ModpackSearchFilters,
): Omit<DiscoverSearch, "q" | "page"> {
  return {
    version: filters.gameVersion ?? undefined,
    loader: filters.loader ?? undefined,
    category: filters.category ?? undefined,
    source: filters.source ?? undefined,
    sort:
      filters.sort === DEFAULT_SEARCH_FILTERS.sort ? undefined : filters.sort,
  };
}

export function parseModpackSource(value: string): ModpackSource | null {
  return pick(value, MODPACK_SOURCES) ?? null;
}

export const SOURCE_LABELS: Record<ModpackSource, string> = {
  modrinth: "Modrinth",
  curseforge: "CurseForge",
};
