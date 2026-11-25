import { describe, expect, it } from "vitest";

import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

import { filterContentEntries } from "./content-table";

function entry(
  title: string,
  kind: InstanceContentKind,
  version: string | null = null,
): InstanceContentEntry {
  return {
    name: `${title.toLowerCase()}.jar`,
    kind,
    enabled: true,
    size: 1,
    title,
    version,
    iconUrl: null,
    fromModpack: false,
  };
}

const ENTRIES = [
  entry("Sodium", "mods", "0.6.5"),
  entry("Faithful", "resourcePacks"),
  entry("Complementary", "shaderPacks"),
];

describe("filterContentEntries", () => {
  it("keeps only the chosen kind", () => {
    expect(
      filterContentEntries(ENTRIES, "resourcePacks", "").map((e) => e.title),
    ).toEqual(["Faithful"]);
    expect(filterContentEntries(ENTRIES, "all", "")).toHaveLength(3);
  });

  it("matches the search against title, file name and version, ignoring case", () => {
    expect(
      filterContentEntries(ENTRIES, "all", "  SODIUM ").map((e) => e.title),
    ).toEqual(["Sodium"]);
    expect(
      filterContentEntries(ENTRIES, "all", "faithful.jar").map((e) => e.title),
    ).toEqual(["Faithful"]);
    expect(
      filterContentEntries(ENTRIES, "all", "0.6").map((e) => e.title),
    ).toEqual(["Sodium"]);
    expect(filterContentEntries(ENTRIES, "mods", "faithful")).toEqual([]);
  });
});
