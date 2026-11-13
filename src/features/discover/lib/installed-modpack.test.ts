import { describe, expect, it } from "vitest";

import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

import { hasInstalledModpack } from "./installed-modpack";

const instance: InstanceEntry = {
  folder: "pack",
  name: "Pack",
  versionId: null,
  minMemoryMb: 512,
  maxMemoryMb: 2048,
  jvmArguments: [],
  fullscreen: false,
  width: null,
  height: null,
  overrideWindow: false,
  overrideMemory: false,
  overrideJava: false,
  overrideJvmArguments: false,
  javaPath: null,
  lastOpenedAt: null,
  iconPath: null,
};

function packInstance(
  projectId: string,
  source: "modrinth" | "curseforge" = "modrinth",
): InstanceEntry {
  return {
    ...instance,
    installation: {
      state: "ready",
      source: {
        type: "modpack",
        source,
        projectId,
        versionId: "v1",
        iconUrl: null,
      },
    },
  };
}

describe("hasInstalledModpack", () => {
  it("matches an instance created from the same store pack", () => {
    expect(
      hasInstalledModpack([packInstance("1KVo5zza")], "modrinth", "1KVo5zza"),
    ).toBe(true);
  });

  it("ignores another pack, another source, and an instance without an installation", () => {
    expect(
      hasInstalledModpack(
        [
          packInstance("other"),
          packInstance("1KVo5zza", "curseforge"),
          instance,
        ],
        "modrinth",
        "1KVo5zza",
      ),
    ).toBe(false);
  });
});
