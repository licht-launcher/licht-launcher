import { describe, expect, it } from "vitest";

import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

import { countContent, modpackOrigin } from "./modpack-content";

function entry(name: string, kind: InstanceContentKind): InstanceContentEntry {
  return {
    name,
    kind,
    enabled: true,
    size: 1,
    title: name,
    version: null,
    iconUrl: null,
    fromModpack: true,
  };
}

describe("modpackOrigin", () => {
  it("keeps store and file modpack sources", () => {
    const store = {
      type: "modpack",
      source: "modrinth",
      projectId: "fo",
      versionId: "v1",
      iconUrl: null,
    } as const;
    expect(modpackOrigin(store)).toBe(store);
    const file = { type: "modpackFile", source: "curseforge" } as const;
    expect(modpackOrigin(file)).toBe(file);
  });

  it("ignores vanilla, loader and missing sources", () => {
    expect(modpackOrigin(undefined)).toBeNull();
    expect(
      modpackOrigin({ type: "vanilla", gameVersion: "1.21.1" }),
    ).toBeNull();
  });
});

describe("countContent", () => {
  it("counts each kind in a fixed order with singular and plural labels", () => {
    expect(
      countContent([
        entry("pack.zip", "resourcePacks"),
        entry("a.jar", "mods"),
        entry("b.jar", "mods"),
        entry("other.zip", "resourcePacks"),
        entry("shader.zip", "shaderPacks"),
      ]),
    ).toEqual([
      { kind: "mods", label: "2 mods" },
      { kind: "resourcePacks", label: "2 resource packs" },
      { kind: "shaderPacks", label: "1 shader pack" },
    ]);
  });

  it("leaves out kinds the modpack does not ship", () => {
    expect(countContent([entry("a.jar", "mods")])).toEqual([
      { kind: "mods", label: "1 mod" },
    ]);
    expect(countContent([])).toEqual([]);
  });
});
