import { describe, expect, it } from "vitest";

import {
  canCreateScratch,
  initialScratchState,
  platformNeedsLoader,
  uploadInstanceName,
} from "@/features/new-instance/model/create-state";

describe("canCreateScratch", () => {
  it("starts on vanilla and cannot create yet", () => {
    expect(initialScratchState.platform).toBe("vanilla");
    expect(canCreateScratch(initialScratchState)).toBe(false);
  });

  it("needs a name and a game version", () => {
    const state = { ...initialScratchState, name: "Survival" };
    expect(canCreateScratch(state)).toBe(false);
    expect(canCreateScratch({ ...state, gameVersion: "1.20.1" })).toBe(true);
    expect(
      canCreateScratch({ ...state, gameVersion: "1.20.1", name: "   " }),
    ).toBe(false);
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
