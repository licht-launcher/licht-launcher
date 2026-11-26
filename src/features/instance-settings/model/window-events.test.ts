import { describe, expect, it } from "vitest";

import {
  parseInstanceSettingsAction,
  parseInstanceSettingsActionFailure,
  parseInstanceSettingsStatus,
} from "@/features/instance-settings/model/window-events";

describe("instance settings window events", () => {
  it("reads the actions an instance settings window can ask for", () => {
    expect(
      parseInstanceSettingsAction({
        kind: "rename",
        folder: "survival",
        name: "Hardcore",
      }),
    ).toEqual({ kind: "rename", folder: "survival", name: "Hardcore" });
    expect(
      parseInstanceSettingsAction({ kind: "delete", folder: "survival" }),
    ).toEqual({ kind: "delete", folder: "survival" });
    expect(
      parseInstanceSettingsAction({
        kind: "changeBase",
        folder: "survival",
        base: {
          platform: "fabric",
          gameVersion: "1.21.1",
          loaderChannel: "manual",
          loaderVersion: "0.16.0",
        },
      }),
    ).toEqual({
      kind: "changeBase",
      folder: "survival",
      base: {
        platform: "fabric",
        gameVersion: "1.21.1",
        loaderChannel: "manual",
        loaderVersion: "0.16.0",
      },
    });
  });

  it("ignores malformed or unknown actions", () => {
    expect(parseInstanceSettingsAction(null)).toBeNull();
    expect(parseInstanceSettingsAction({ kind: "delete" })).toBeNull();
    expect(
      parseInstanceSettingsAction({ kind: "format", folder: "survival" }),
    ).toBeNull();
    expect(
      parseInstanceSettingsAction({
        kind: "rename",
        folder: "survival",
        name: "",
      }),
    ).toBeNull();
    expect(
      parseInstanceSettingsAction({
        kind: "changeBase",
        folder: "survival",
        base: {
          platform: "bukkit",
          gameVersion: "1.21.1",
          loaderChannel: "stable",
          loaderVersion: null,
        },
      }),
    ).toBeNull();
  });

  it("reads the launcher status and action failures", () => {
    expect(
      parseInstanceSettingsStatus({
        busy: true,
        versionBusy: false,
        activeFolders: ["survival"],
      }),
    ).toEqual({ busy: true, versionBusy: false, activeFolders: ["survival"] });
    expect(
      parseInstanceSettingsStatus({
        busy: true,
        versionBusy: false,
        activeFolders: [1],
      }),
    ).toBeNull();
    expect(
      parseInstanceSettingsActionFailure({
        folder: "survival",
        message: "Disk full",
      }),
    ).toEqual({ folder: "survival", message: "Disk full" });
    expect(parseInstanceSettingsActionFailure({ folder: "" })).toBeNull();
  });
});
