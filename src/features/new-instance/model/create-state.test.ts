import { describe, expect, it } from "vitest";

import {
  canCreate,
  fileBaseName,
  initialCreateState,
  platformNeedsLoader,
} from "@/features/new-instance/model/create-state";

describe("canCreate", () => {
  it("starts on the scratch source and cannot create yet", () => {
    expect(initialCreateState.source).toBe("scratch");
    expect(canCreate(initialCreateState)).toBe(false);
  });

  it("needs a name and a game version from scratch", () => {
    const state = { ...initialCreateState, name: "Survival" };
    expect(canCreate(state)).toBe(false);
    expect(canCreate({ ...state, gameVersion: "1.20.1" })).toBe(true);
    expect(canCreate({ ...state, gameVersion: "1.20.1", name: "   " })).toBe(
      false,
    );
  });

  it("requires a manual loader version only for loader platforms", () => {
    const state = {
      ...initialCreateState,
      name: "Modded",
      gameVersion: "1.20.1",
      platform: "fabric" as const,
      loaderChannel: "manual" as const,
    };
    expect(canCreate(state)).toBe(false);
    expect(canCreate({ ...state, loaderVersion: "0.16.0" })).toBe(true);
    expect(canCreate({ ...state, platform: "vanilla" })).toBe(true);
  });

  it("needs a modpack, one of its versions and a name", () => {
    const pack = {
      id: "ABC",
      source: "modrinth" as const,
      name: "Cobblemon",
      description: "",
      downloads: 1,
      iconUrl: null,
      author: "team",
    };
    const version = { id: "v1", name: "1.0.0", gameVersions: [], loaders: [] };
    const state = {
      ...initialCreateState,
      source: "modpack" as const,
      name: "Cobblemon",
    };
    expect(canCreate(state)).toBe(false);
    expect(canCreate({ ...state, modpack: pack })).toBe(false);
    expect(
      canCreate({ ...state, modpack: pack, modpackVersion: version }),
    ).toBe(true);
    expect(
      canCreate({
        ...state,
        modpack: pack,
        modpackVersion: version,
        name: "",
      }),
    ).toBe(false);
  });

  it("needs a picked file and a name", () => {
    const state = { ...initialCreateState, source: "file" as const };
    expect(canCreate(state)).toBe(false);
    expect(canCreate({ ...state, filePath: "C:\\packs\\a.mrpack" })).toBe(
      false,
    );
    expect(
      canCreate({ ...state, filePath: "C:\\packs\\a.mrpack", name: "Pack" }),
    ).toBe(true);
  });

  it("needs a source instance and a name to clone", () => {
    const state = { ...initialCreateState, source: "clone" as const };
    expect(canCreate(state)).toBe(false);
    expect(canCreate({ ...state, cloneFolder: "Survival" })).toBe(false);
    expect(
      canCreate({ ...state, cloneFolder: "Survival", name: "Survival copy" }),
    ).toBe(true);
  });
});

describe("helpers", () => {
  it("derives a default name from the archive file name", () => {
    expect(fileBaseName("C:\\packs\\Cobble Pack.mrpack")).toBe("Cobble Pack");
    expect(fileBaseName("/home/a/pack.v2.zip")).toBe("pack.v2");
    expect(fileBaseName("/home/a/noextension")).toBe("noextension");
  });

  it("knows which platforms need a loader", () => {
    expect(platformNeedsLoader("vanilla")).toBe(false);
    expect(platformNeedsLoader("fabric")).toBe(true);
  });
});
