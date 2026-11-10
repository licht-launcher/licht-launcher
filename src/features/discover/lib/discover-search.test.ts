import { describe, expect, it } from "vitest";

import {
  DEFAULT_SEARCH_FILTERS,
  filterParams,
  parseDiscoverSearch,
  parseModpackSource,
  searchFilters,
} from "./discover-search";

const EMPTY = {
  q: undefined,
  page: undefined,
  version: undefined,
  loader: undefined,
  category: undefined,
  source: undefined,
  sort: undefined,
};

describe("parseDiscoverSearch", () => {
  it("keeps a trimmed query and a page after the first", () => {
    expect(parseDiscoverSearch({ q: "  cobble ", page: 3 })).toEqual({
      ...EMPTY,
      q: "cobble",
      page: 3,
    });
  });

  it("drops empty queries and unusable pages", () => {
    for (const page of [1, 0, -2, 1.5, "x", undefined]) {
      expect(parseDiscoverSearch({ q: "  ", page })).toEqual(EMPTY);
    }
  });

  it("keeps known filter values", () => {
    expect(
      parseDiscoverSearch({
        version: " 1.20.1 ",
        loader: "neoforge",
        category: "kitchenSink",
        source: "curseforge",
        sort: "updated",
      }),
    ).toEqual({
      ...EMPTY,
      version: "1.20.1",
      loader: "neoforge",
      category: "kitchenSink",
      source: "curseforge",
      sort: "updated",
    });
  });

  it("drops unknown filter values and the default sort", () => {
    expect(
      parseDiscoverSearch({
        version: "  ",
        loader: "rift",
        category: "pvp",
        source: "ftb",
        sort: "downloads",
      }),
    ).toEqual(EMPTY);
    expect(parseDiscoverSearch({ sort: "name" })).toEqual(EMPTY);
  });
});

describe("searchFilters and filterParams", () => {
  it("turn URL state into search filters and back", () => {
    const filters = searchFilters({
      q: "x",
      page: 2,
      version: "1.20.1",
      loader: "fabric",
      category: "magic",
      source: "modrinth",
      sort: "relevance",
    });
    expect(filters).toEqual({
      gameVersion: "1.20.1",
      loader: "fabric",
      category: "magic",
      source: "modrinth",
      sort: "relevance",
    });
    expect(filterParams(filters)).toEqual({
      version: "1.20.1",
      loader: "fabric",
      category: "magic",
      source: "modrinth",
      sort: "relevance",
    });
  });

  it("leave defaults out of the URL", () => {
    expect(searchFilters({})).toEqual(DEFAULT_SEARCH_FILTERS);
    expect(filterParams(DEFAULT_SEARCH_FILTERS)).toEqual({
      version: undefined,
      loader: undefined,
      category: undefined,
      source: undefined,
      sort: undefined,
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
