import { describe, expect, it } from "vitest";

import { parseDiscoverSearch, parseModpackSource } from "./discover-search";

describe("parseDiscoverSearch", () => {
  it("keeps a trimmed query and a page after the first", () => {
    expect(parseDiscoverSearch({ q: "  cobble ", page: 3 })).toEqual({
      q: "cobble",
      page: 3,
    });
  });

  it("drops empty queries and unusable pages", () => {
    for (const page of [1, 0, -2, 1.5, "x", undefined]) {
      expect(parseDiscoverSearch({ q: "  ", page })).toEqual({
        q: undefined,
        page: undefined,
      });
    }
  });
});

describe("parseModpackSource", () => {
  it("accepts only known sources", () => {
    expect(parseModpackSource("modrinth")).toBe("modrinth");
    expect(parseModpackSource("curseforge")).toBe("curseforge");
    expect(parseModpackSource("ftb")).toBeNull();
  });
});
