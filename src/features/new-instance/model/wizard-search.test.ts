import { describe, expect, it } from "vitest";
import { initialWizardState } from "@/features/new-instance/model/wizard";
import {
  parseNewInstanceSearch,
  patchSearch,
  searchFromWizardState,
  wizardStateFromSearch,
} from "@/features/new-instance/model/wizard-search";

describe("wizard-search", () => {
  it("parses and clamps the step to the source path", () => {
    expect(
      parseNewInstanceSearch({
        step: "details",
        source: "modpack",
      }),
    ).toMatchObject({ step: "source", source: "modpack" });

    expect(
      parseNewInstanceSearch({
        step: "details",
        source: "scratch",
        platform: "vanilla",
        name: "Survival",
        gameVersion: "1.20.1",
      }),
    ).toMatchObject({
      step: "details",
      source: "scratch",
      platform: "vanilla",
      name: "Survival",
      gameVersion: "1.20.1",
    });
  });

  it("round-trips scratch fields through search", () => {
    const state = {
      ...initialWizardState,
      step: "details" as const,
      source: "scratch" as const,
      platform: "vanilla" as const,
      name: "Survival",
      gameVersion: "1.20.1",
      includeSnapshots: true,
    };
    const search = searchFromWizardState(state);
    expect(wizardStateFromSearch(search, null, null)).toEqual(state);
  });

  it("round-trips the picked archive through search", () => {
    const state = {
      ...initialWizardState,
      step: "file" as const,
      source: "file" as const,
      name: "Cobblemon",
      filePath: "C:\\packs\\cobble.mrpack",
    };
    const search = searchFromWizardState(state);
    expect(parseNewInstanceSearch({ ...search })).toEqual(search);
    expect(wizardStateFromSearch(search, null, null)).toEqual(state);
  });

  it("patches search without dropping unrelated fields", () => {
    const current = parseNewInstanceSearch({
      step: "details",
      source: "scratch",
      platform: "vanilla",
      name: "Survival",
      gameVersion: "1.20.1",
    });
    expect(patchSearch(current, { name: "Hardcore" }).name).toBe("Hardcore");
    expect(patchSearch(current, { name: "Hardcore" }).gameVersion).toBe(
      "1.20.1",
    );
  });
});
