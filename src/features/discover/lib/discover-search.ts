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

/** No narrowing: every source, sorted by relevance. */
export const DEFAULT_SEARCH_FILTERS: ModpackSearchFilters = {
  gameVersion: null,
  loaders: [],
  categories: [],
  source: null,
  sort: "relevance",
};

/** Discover's URL state; defaults are left out so links stay short. */
export type DiscoverSearch = {
  q?: string;
  version?: string;
  loaders?: ModpackLoader[];
  categories?: ModpackCategory[];
  source?: ModpackSource;
  sort?: ModpackSort;
};

const MODPACK_SOURCES: readonly ModpackSource[] = ["modrinth", "curseforge"];

function pick<T extends string>(
  value: unknown,
  allowed: readonly T[],
): T | undefined {
  return allowed.find((option) => option === value);
}

/** The allowed values `value` lists, in `allowed` order and without repeats. */
function pickAll<T extends string>(
  value: unknown,
  allowed: readonly T[],
): T[] | undefined {
  if (!Array.isArray(value)) return undefined;
  const picked = allowed.filter((option) => value.includes(option));
  return picked.length === 0 ? undefined : picked;
}

function values<T extends string>(options: readonly { value: T }[]): T[] {
  return options.map((option) => option.value);
}

/** Reads Discover's URL state, dropping anything that is not usable. */
export function parseDiscoverSearch(
  search: Record<string, unknown>,
): DiscoverSearch {
  const q = typeof search.q === "string" ? search.q.trim() : "";
  const version =
    typeof search.version === "string" ? search.version.trim() : "";
  const sort = pick(search.sort, values(SORT_OPTIONS));
  return {
    q: q === "" ? undefined : q,
    version: version === "" ? undefined : version,
    loaders: chosenLoaders(search),
    categories: pickAll(search.categories, values(CATEGORY_OPTIONS)),
    source: pick(search.source, MODPACK_SOURCES),
    sort: sort === DEFAULT_SEARCH_FILTERS.sort ? undefined : sort,
  };
}

/** The search filters a Discover URL asks for. */
export function searchFilters(search: DiscoverSearch): ModpackSearchFilters {
  return {
    gameVersion: search.version ?? null,
    loaders: search.loaders ?? [],
    categories: search.categories ?? [],
    source: search.source ?? null,
    sort: search.sort ?? DEFAULT_SEARCH_FILTERS.sort,
  };
}

/** URL params for `filters`; defaults become `undefined` and drop out. */
export function filterParams(
  filters: ModpackSearchFilters,
): Omit<DiscoverSearch, "q"> {
  return {
    version: filters.gameVersion ?? undefined,
    loaders: filters.loaders.length === 0 ? undefined : [...filters.loaders],
    categories:
      filters.categories.length === 0 ? undefined : [...filters.categories],
    source: filters.source ?? undefined,
    sort:
      filters.sort === DEFAULT_SEARCH_FILTERS.sort ? undefined : filters.sort,
  };
}

/** The loaders in `search`, in sidebar order. A lone `loader` is an old link. */
function chosenLoaders(
  search: Record<string, unknown>,
): ModpackLoader[] | undefined {
  const listed = pickAll(search.loaders, values(LOADER_OPTIONS));
  if (listed) return listed;
  const loader = pick(search.loader, values(LOADER_OPTIONS));
  return loader === undefined ? undefined : [loader];
}

/**
 * Adds `loader` to the chosen ones, or removes it when already chosen. The
 * result keeps the sidebar's order so equal choices make equal searches.
 */
export function toggleLoader(
  loaders: readonly ModpackLoader[],
  loader: ModpackLoader,
): ModpackLoader[] {
  const chosen = loaders.includes(loader);
  return values(LOADER_OPTIONS).filter((option) =>
    option === loader ? !chosen : loaders.includes(option),
  );
}

/**
 * Adds `category` to the chosen ones, or removes it when already chosen. The
 * result keeps the sidebar's order so equal choices make equal searches.
 */
export function toggleCategory(
  categories: readonly ModpackCategory[],
  category: ModpackCategory,
): ModpackCategory[] {
  const chosen = categories.includes(category);
  return values(CATEGORY_OPTIONS).filter((option) =>
    option === category ? !chosen : categories.includes(option),
  );
}

/** Categories CurseForge has no modpack category for (see docs/integrations/CURSEFORGE.md). */
const CURSEFORGE_MISSING_CATEGORIES: readonly ModpackCategory[] = [
  "optimization",
];

/** Whether a search limited to `source` can narrow by `category`. */
export function categorySearchable(
  category: ModpackCategory,
  source: ModpackSource | null,
): boolean {
  return (
    source !== "curseforge" || !CURSEFORGE_MISSING_CATEGORIES.includes(category)
  );
}

/** `filters` limited to `source`, without categories that source cannot search. */
export function withSource(
  filters: ModpackSearchFilters,
  source: ModpackSource | null,
): ModpackSearchFilters {
  return {
    ...filters,
    source,
    categories: filters.categories.filter((category) =>
      categorySearchable(category, source),
    ),
  };
}

export function parseModpackSource(value: string): ModpackSource | null {
  return pick(value, MODPACK_SOURCES) ?? null;
}

export const SOURCE_LABELS: Record<ModpackSource, string> = {
  modrinth: "Modrinth",
  curseforge: "CurseForge",
};
