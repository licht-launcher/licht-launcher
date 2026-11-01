import { describe, expect, it } from "vitest";

import {
  canAdvance,
  fileBaseName,
  initialWizardState,
  nextStep,
  previousStep,
  stepsFor,
} from "@/features/new-instance/model/wizard";

describe("wizard steps", () => {
  it("starts with the scratch source selected so Continue is ready", () => {
    expect(initialWizardState.source).toBe("scratch");
    expect(stepsFor(initialWizardState)).toEqual([
      "source",
      "platform",
      "details",
      "confirm",
    ]);
    expect(canAdvance(initialWizardState)).toBe(true);
  });

  it("builds the vanilla scratch path", () => {
    const state = {
      ...initialWizardState,
      source: "scratch" as const,
      platform: "vanilla" as const,
    };
    expect(stepsFor(state)).toEqual([
      "source",
      "platform",
      "details",
      "confirm",
    ]);
  });

  it("keeps loader choices on the details step for fabric", () => {
    const state = {
      ...initialWizardState,
      source: "scratch" as const,
      platform: "fabric" as const,
    };
    expect(stepsFor(state)).toEqual([
      "source",
      "platform",
      "details",
      "confirm",
    ]);
  });

  it("requires a manual loader version on details", () => {
    const state = {
      ...initialWizardState,
      step: "details" as const,
      source: "scratch" as const,
      platform: "fabric" as const,
      name: "Modded",
      gameVersion: "1.20.1",
      loaderChannel: "manual" as const,
      loaderVersion: "",
    };
    expect(canAdvance(state)).toBe(false);
    expect(canAdvance({ ...state, loaderVersion: "0.16.0" })).toBe(true);
  });

  it("builds the modpack path", () => {
    const state = {
      ...initialWizardState,
      source: "modpack" as const,
    };
    expect(stepsFor(state)).toEqual([
      "source",
      "modpack",
      "modpack-version",
      "confirm",
    ]);
  });

  it("builds the local file path", () => {
    const state = {
      ...initialWizardState,
      source: "file" as const,
    };
    expect(stepsFor(state)).toEqual(["source", "file", "confirm"]);
  });

  it("needs a picked file and a name to leave the file step", () => {
    const state = {
      ...initialWizardState,
      step: "file" as const,
      source: "file" as const,
    };
    expect(canAdvance(state)).toBe(false);
    expect(canAdvance({ ...state, filePath: "C:\\packs\\a.mrpack" })).toBe(
      false,
    );
    expect(
      canAdvance({ ...state, filePath: "C:\\packs\\a.mrpack", name: "  " }),
    ).toBe(false);
    expect(
      canAdvance({ ...state, filePath: "C:\\packs\\a.mrpack", name: "Pack" }),
    ).toBe(true);
  });

  it("derives a default name from the archive file name", () => {
    expect(fileBaseName("C:\\packs\\Cobble Pack.mrpack")).toBe("Cobble Pack");
    expect(fileBaseName("/home/a/pack.v2.zip")).toBe("pack.v2");
    expect(fileBaseName("/home/a/noextension")).toBe("noextension");
  });

  it("advances and goes back", () => {
    let state = { ...initialWizardState, source: "scratch" as const };
    expect(canAdvance(state)).toBe(true);
    state = { ...state, step: nextStep(state) };
    expect(state.step).toBe("platform");
    state = { ...state, step: previousStep(state) };
    expect(state.step).toBe("source");
  });
});
