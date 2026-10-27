import { describe, expect, it } from "vitest";

import {
  type InstanceSort,
  sortInstances,
} from "@/features/instances/lib/sort-instances";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

function entry(
  folder: string,
  name: string,
  lastOpenedAt: string | null = null,
): InstanceEntry {
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
    lastOpenedAt,
    iconPath: null,
  };
}

describe("sortInstances", () => {
  it.each<{ sort: InstanceSort; names: string[] }>([
    { sort: "name", names: ["Alpha", "Classic", "Zeta"] },
    { sort: "nameDesc", names: ["Zeta", "Classic", "Alpha"] },
  ])(
    "sorts by $sort without modifying the original list",
    ({ sort, names }) => {
      const instances = [
        entry("zeta", "Zeta"),
        entry("alpha", "Alpha"),
        entry("classic", "Classic"),
      ];
      expect(sortInstances(instances, sort).map((item) => item.name)).toEqual(
        names,
      );
      expect(instances.map((item) => item.name)).toEqual([
        "Zeta",
        "Alpha",
        "Classic",
      ]);
    },
  );

  it("orders by last opened time, then never opened by name", () => {
    const instances = [
      entry("zeta", "Zeta", "2026-01-01T10:00:00.000Z"),
      entry("alpha", "Alpha", null),
      entry("classic", "Classic", "2026-03-01T10:00:00.000Z"),
      entry("beta", "Beta", null),
      entry("delta", "Delta", "2026-02-01T10:00:00.000Z"),
    ];
    expect(
      sortInstances(instances, "lastPlayed").map((item) => item.name),
    ).toEqual(["Classic", "Delta", "Zeta", "Alpha", "Beta"]);
  });

  it("falls back to name order when nothing was opened", () => {
    const instances = [
      entry("zeta", "Zeta"),
      entry("alpha", "Alpha"),
      entry("classic", "Classic"),
    ];
    expect(
      sortInstances(instances, "lastPlayed").map((item) => item.name),
    ).toEqual(["Alpha", "Classic", "Zeta"]);
  });

  it("breaks ties by name when opened at the same time", () => {
    const stamp = "2026-03-01T10:00:00.000Z";
    const instances = [
      entry("zeta", "Zeta", stamp),
      entry("alpha", "Alpha", stamp),
    ];
    expect(
      sortInstances(instances, "lastPlayed").map((item) => item.name),
    ).toEqual(["Alpha", "Zeta"]);
  });
});
