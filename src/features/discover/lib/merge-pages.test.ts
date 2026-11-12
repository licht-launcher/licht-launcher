import { describe, expect, it } from "vitest";

import {
  modpackListItem,
  modpackSearchPage,
} from "@/features/discover/fixtures/modpacks";

import { mergeSearchPages, modpackKey } from "./merge-pages";

describe("mergeSearchPages", () => {
  it("joins pages in order and drops repeats", () => {
    const first = modpackListItem({ id: "a" });
    const second = modpackListItem({ id: "b" });
    const third = modpackListItem({ id: "c" });

    const items = mergeSearchPages([
      modpackSearchPage({ items: [first, second], page: 1 }),
      modpackSearchPage({ items: [second, third], page: 2 }),
    ]);

    expect(items.map(modpackKey)).toEqual([
      "modrinth:a",
      "modrinth:b",
      "modrinth:c",
    ]);
  });

  it("keeps the same id from different sources apart", () => {
    const items = mergeSearchPages([
      modpackSearchPage({
        items: [
          modpackListItem({ id: "1", source: "modrinth" }),
          modpackListItem({ id: "1", source: "curseforge" }),
        ],
      }),
    ]);

    expect(items).toHaveLength(2);
  });
});
