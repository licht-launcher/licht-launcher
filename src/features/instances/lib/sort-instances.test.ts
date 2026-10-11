import { describe, expect, it } from "vitest";

import { sortInstances } from "@/features/instances/lib/sort-instances";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

function entry(folder: string, name: string): InstanceEntry {
  return {
    folder,
    name,
    versionId: "1.20.1",
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
  };
}

describe("sortInstances", () => {
  const instances = [
    entry("zeta", "Zeta"),
    entry("alpha", "Alpha"),
    entry("classic", "Classic"),
  ];

  it("sorts by name", () => {
    expect(
      sortInstances(instances, "name", "classic").map((item) => item.name),
    ).toEqual(["Alpha", "Classic", "Zeta"]);
  });

  it("puts the last played folder first, then sorts by name", () => {
    expect(
      sortInstances(instances, "lastPlayed", "classic").map(
        (item) => item.name,
      ),
    ).toEqual(["Classic", "Alpha", "Zeta"]);
  });

  it("falls back to name order when nothing was played", () => {
    expect(
      sortInstances(instances, "lastPlayed", null).map((item) => item.name),
    ).toEqual(["Alpha", "Classic", "Zeta"]);
  });
});
