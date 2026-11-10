import { describe, expect, it } from "vitest";

import { paginationItems } from "./pagination-items";

describe("paginationItems", () => {
  it("lists every page when they all fit", () => {
    expect(paginationItems(1, 1)).toEqual([1]);
    expect(paginationItems(3, 7)).toEqual([1, 2, 3, 4, 5, 6, 7]);
  });

  it("keeps the first pages together near the start", () => {
    expect(paginationItems(1, 20)).toEqual([1, 2, 3, 4, 5, "end-gap", 20]);
  });

  it("surrounds the current page with gaps in the middle", () => {
    expect(paginationItems(10, 20)).toEqual([
      1,
      "start-gap",
      9,
      10,
      11,
      "end-gap",
      20,
    ]);
  });

  it("keeps the last pages together near the end", () => {
    expect(paginationItems(20, 20)).toEqual([
      1,
      "start-gap",
      16,
      17,
      18,
      19,
      20,
    ]);
  });
});
