import { describe, expect, it } from "vitest";

import {
  canCreateScratch,
  defaultInstanceName,
  initialScratchState,
  platformNeedsLoader,
  resolveInstanceName,
  uniqueName,
  uploadInstanceName,
} from "@/features/new-instance/model/create-state";

describe("canCreateScratch", () => {
  it("starts on vanilla and cannot create yet", () => {
    expect(initialScratchState.platform).toBe("vanilla");
    expect(canCreateScratch(initialScratchState)).toBe(false);
  });

  it("needs a game version but not a typed name", () => {
    const state = { ...initialScratchState, name: "Survival" };
    expect(canCreateScratch(state)).toBe(false);
    expect(canCreateScratch({ ...state, gameVersion: "1.20.1" })).toBe(true);
    expect(
      canCreateScratch({ ...state, gameVersion: "1.20.1", name: "" }),
    ).toBe(true);
  });

  it("requires a manual loader version only for loader platforms", () => {
    const state = {
      ...initialScratchState,
      name: "Modded",
      gameVersion: "1.20.1",
      platform: "fabric" as const,
      loaderChannel: "manual" as const,
    };
    expect(canCreateScratch(state)).toBe(false);
    expect(canCreateScratch({ ...state, loaderVersion: "0.16.0" })).toBe(true);
    expect(canCreateScratch({ ...state, platform: "vanilla" })).toBe(true);
  });
});

describe("instance names", () => {
  it("defaults to the platform and the game version", () => {
    const state = { ...initialScratchState, gameVersion: "26.3" };
    expect(defaultInstanceName(state)).toBe("Vanilla 26.3");
    expect(defaultInstanceName({ ...state, platform: "fabric" })).toBe(
      "Fabric 26.3",
    );
    expect(defaultInstanceName({ ...state, platform: "neoforge" })).toBe(
      "NeoForge 26.3",
    );
  });

  it("has no default until a game version is chosen", () => {
    expect(defaultInstanceName(initialScratchState)).toBe("");
  });

  it("uses the typed name when there is one and the default otherwise", () => {
    const state = {
      ...initialScratchState,
      platform: "fabric" as const,
      gameVersion: "1.20.1",
    };
    expect(resolveInstanceName({ ...state, name: "  Survival  " })).toBe(
      "Survival",
    );
    expect(resolveInstanceName({ ...state, name: "   " })).toBe(
      "Fabric 1.20.1",
    );
    expect(resolveInstanceName(state)).toBe("Fabric 1.20.1");
  });
});

describe("unique names", () => {
  it("keeps a free name as it is", () => {
    expect(uniqueName("Fabric 26.3", [])).toBe("Fabric 26.3");
    expect(uniqueName("Fabric 26.3", ["Vanilla 26.3"])).toBe("Fabric 26.3");
  });

  it("numbers a name that is already used, starting at (1)", () => {
    expect(uniqueName("Fabric 26.3", ["Fabric 26.3"])).toBe("Fabric 26.3 (1)");
    expect(uniqueName("Fabric 26.3", ["Fabric 26.3", "Fabric 26.3 (1)"])).toBe(
      "Fabric 26.3 (2)",
    );
  });

  it("reuses the first free number and ignores case and spacing", () => {
    expect(uniqueName("Pack", ["pack ", "Pack (2)"])).toBe("Pack (1)");
  });

  it("only numbers a name that is really the same", () => {
    expect(uniqueName("Pack", ["Pack!"])).toBe("Pack");
    expect(uniqueName("Pack (1)", ["Pack 1"])).toBe("Pack (1)");
    expect(uniqueName("My-Pack", ["My Pack"])).toBe("My-Pack");
    expect(uniqueName("Pack", ["pack "])).toBe("Pack (1)");
  });

  it("numbers the default instance name when it is taken", () => {
    const state = {
      ...initialScratchState,
      platform: "fabric" as const,
      gameVersion: "26.3",
    };
    expect(defaultInstanceName(state, ["Fabric 26.3"])).toBe("Fabric 26.3 (1)");
    expect(resolveInstanceName(state, ["Fabric 26.3"])).toBe("Fabric 26.3 (1)");
  });

  it("keeps a typed name as typed, even when it is taken", () => {
    const state = {
      ...initialScratchState,
      name: " Survival ",
      gameVersion: "1",
    };
    expect(resolveInstanceName(state, ["Survival"])).toBe("Survival");
  });

  it("blocks creating while the loader has no usable build", () => {
    const state = {
      ...initialScratchState,
      platform: "fabric" as const,
      gameVersion: "26.3",
    };
    expect(canCreateScratch(state, true)).toBe(true);
    expect(canCreateScratch(state, false)).toBe(false);
  });

  it("allows a typed name that another instance already has", () => {
    const state = {
      ...initialScratchState,
      name: "survival",
      gameVersion: "1",
    };
    expect(canCreateScratch(state)).toBe(true);
  });
});

describe("helpers", () => {
  it("names an uploaded instance after the archive", () => {
    expect(uploadInstanceName("C:\\packs\\Cobble Pack.mrpack")).toBe(
      "Cobble Pack",
    );
    expect(uploadInstanceName("/home/a/pack.v2.zip")).toBe("pack.v2");
    expect(uploadInstanceName("/home/a/noextension")).toBe("noextension");
  });

  it("falls back to a generic name when the file name is empty", () => {
    expect(uploadInstanceName("/home/a/.zip")).toBe("Imported pack");
    expect(uploadInstanceName("/home/a/")).toBe("Imported pack");
  });

  it("knows which platforms need a loader", () => {
    expect(platformNeedsLoader("vanilla")).toBe(false);
    expect(platformNeedsLoader("fabric")).toBe(true);
    expect(platformNeedsLoader(null)).toBe(false);
  });
});
