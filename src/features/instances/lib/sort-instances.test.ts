import { describe, expect, it } from "vitest";

import { sortInstances } from "@/features/instances/lib/sort-instances";
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
  it("sorts by name", () => {
    const instances = [
      entry("zeta", "Zeta"),
      entry("alpha", "Alpha"),
      entry("classic", "Classic"),
    ];
    expect(sortInstances(instances, "name").map((item) => item.name)).toEqual([
      "Alpha",
      "Classic",
      "Zeta",
    ]);
  });

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
