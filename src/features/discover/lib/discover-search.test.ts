import { describe, expect, it } from "vitest";

import {
  DEFAULT_SEARCH_FILTERS,
  filterParams,
  parseDiscoverSearch,
  parseModpackSource,
  searchFilters,
  toggleCategory,
  toggleLoader,
  withSource,
} from "./discover-search";

const EMPTY = {
  q: undefined,
  version: undefined,
  loaders: undefined,
  categories: undefined,
  source: undefined,
  sort: undefined,
};

describe("parseDiscoverSearch", () => {
  it("keeps a trimmed query", () => {
    expect(parseDiscoverSearch({ q: "  cobble " })).toEqual({
      ...EMPTY,
      q: "cobble",
    });
  });

  it("drops empty queries and the page of old paginated links", () => {
    expect(parseDiscoverSearch({ q: "  ", page: 3 })).toEqual(EMPTY);
  });

  it("keeps known filter values", () => {
    expect(
      parseDiscoverSearch({
        version: " 1.20.1 ",
        loaders: ["neoforge", "fabric", "rift"],
        categories: ["quests", "kitchenSink", "quests", "pvp"],
        source: "curseforge",
        sort: "updated",
      }),
    ).toEqual({
      ...EMPTY,
      version: "1.20.1",
      loaders: ["fabric", "neoforge"],
      // Known categories only, once each, in the sidebar's order.
      categories: ["kitchenSink", "quests"],
      source: "curseforge",
      sort: "updated",
    });
  });

  it("drops unknown filter values and the default sort", () => {
    expect(
      parseDiscoverSearch({
        version: "  ",
        loader: "rift",
        loaders: ["rift"],
        categories: ["pvp"],
        source: "ftb",
        sort: "relevance",
      }),
    ).toEqual(EMPTY);
    expect(parseDiscoverSearch({ sort: "name" })).toEqual(EMPTY);
    expect(parseDiscoverSearch({ categories: "magic" })).toEqual(EMPTY);
  });
});

describe("searchFilters and filterParams", () => {
  it("turn URL state into search filters and back", () => {
    const filters = searchFilters({
      q: "x",
      version: "1.20.1",
      loaders: ["fabric"],
      categories: ["magic", "quests"],
      source: "modrinth",
      sort: "downloads",
    });
    expect(filters).toEqual({
      gameVersion: "1.20.1",
      loaders: ["fabric"],
      categories: ["magic", "quests"],
      source: "modrinth",
      sort: "downloads",
    });
    expect(filterParams(filters)).toEqual({
      version: "1.20.1",
      loaders: ["fabric"],
      categories: ["magic", "quests"],
      source: "modrinth",
      sort: "downloads",
    });
  });

  it("leave defaults out of the URL", () => {
    expect(searchFilters({})).toEqual(DEFAULT_SEARCH_FILTERS);
    expect(filterParams(DEFAULT_SEARCH_FILTERS)).toEqual({
      version: undefined,
      loaders: undefined,
      categories: undefined,
      source: undefined,
      sort: undefined,
    });
  });
});

describe("toggleLoader", () => {
  it("adds a loader in the sidebar's order", () => {
    expect(toggleLoader(["quilt"], "fabric")).toEqual(["fabric", "quilt"]);
  });

  it("removes a chosen loader", () => {
    expect(toggleLoader(["fabric", "quilt"], "fabric")).toEqual(["quilt"]);
  });
});

describe("parseDiscoverSearch legacy loader", () => {
  it("reads a single loader from an older link", () => {
    expect(parseDiscoverSearch({ loader: "quilt" })).toEqual({
      ...EMPTY,
      loaders: ["quilt"],
    });
  });
});

describe("toggleCategory", () => {
  it("adds a category in the sidebar's order", () => {
    expect(toggleCategory(["technology"], "magic")).toEqual([
      "magic",
      "technology",
    ]);
  });

  it("removes a chosen category", () => {
    expect(toggleCategory(["magic", "technology"], "magic")).toEqual([
      "technology",
    ]);
  });
});

describe("withSource", () => {
  it("keeps every category for any source or Modrinth", () => {
    const filters = {
      ...DEFAULT_SEARCH_FILTERS,
      categories: ["magic" as const, "optimization" as const],
    };
    expect(withSource(filters, null).categories).toEqual([
      "magic",
      "optimization",
    ]);
    expect(withSource(filters, "modrinth")).toEqual({
      ...filters,
      source: "modrinth",
    });
  });

  it("drops categories CurseForge cannot search", () => {
    expect(
      withSource(
        { ...DEFAULT_SEARCH_FILTERS, categories: ["magic", "optimization"] },
        "curseforge",
      ),
    ).toEqual({
      ...DEFAULT_SEARCH_FILTERS,
      source: "curseforge",
      categories: ["magic"],
    });
  });
});

describe("parseModpackSource", () => {
  it("accepts only known sources", () => {
    expect(parseModpackSource("modrinth")).toBe("modrinth");
    expect(parseModpackSource("curseforge")).toBe("curseforge");
    expect(parseModpackSource("ftb")).toBeNull();
  });
});
