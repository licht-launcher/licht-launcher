import { describe, expect, it } from "vitest";

import {
  createVersionChoices,
  filterVersionGroups,
  groupVersionsByMajor,
  updateNameForMajor,
  versionMajorGroup,
} from "@/features/instances/model/create-versions";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

describe("createVersionChoices", () => {
  const versions: VersionSummary[] = [
    { id: "1.20.1", versionType: "release", installed: false },
    { id: "24w14a", versionType: "snapshot", installed: false },
    { id: "b1.8", versionType: "old_beta", installed: true },
  ];

  it("keeps releases by default", () => {
    expect(
      createVersionChoices(versions, false).map((item) => item.id),
    ).toEqual(["1.20.1"]);
  });

  it("adds snapshots when requested", () => {
    expect(createVersionChoices(versions, true).map((item) => item.id)).toEqual(
      ["1.20.1", "24w14a"],
    );
  });
});

describe("groupVersionsByMajor", () => {
  it("groups by major.minor and keeps order", () => {
    const versions: VersionSummary[] = [
      { id: "1.21.1", versionType: "release", installed: false },
      { id: "1.21", versionType: "release", installed: false },
      { id: "1.20.1", versionType: "release", installed: false },
      { id: "24w14a", versionType: "snapshot", installed: false },
    ];

    expect(versionMajorGroup("1.21.1")).toBe("1.21");
    expect(versionMajorGroup("24w14a")).toBe("Snapshots");
    expect(
      groupVersionsByMajor(versions).map((group) => ({
        major: group.major,
        ids: group.versions.map((version) => version.id),
      })),
    ).toEqual([
      { major: "1.21", ids: ["1.21.1", "1.21"] },
      { major: "1.20", ids: ["1.20.1"] },
      { major: "Snapshots", ids: ["24w14a"] },
    ]);
  });

  it("resolves known update names", () => {
    expect(updateNameForMajor("26.3")).toBe("Wilderness Bound");
    expect(updateNameForMajor("26.2")).toBe("Chaos Cubed");
    expect(updateNameForMajor("26.1")).toBe("Tiny Takeover");
    expect(updateNameForMajor("1.21")).toBe("Tricky Trials");
    expect(updateNameForMajor("1.20")).toBe("Trails & Tales");
    expect(updateNameForMajor("Snapshots")).toBeNull();
    expect(updateNameForMajor("0.0")).toBeNull();
  });

  it("groups year-based releases by drop", () => {
    expect(versionMajorGroup("26.1")).toBe("26.1");
    expect(versionMajorGroup("26.1.2")).toBe("26.1");
    expect(versionMajorGroup("26.3")).toBe("26.3");
  });

  it("filters groups by version id query", () => {
    const groups = groupVersionsByMajor([
      { id: "1.21.1", versionType: "release", installed: false },
      { id: "1.21", versionType: "release", installed: false },
      { id: "1.20.1", versionType: "release", installed: false },
      { id: "24w14a", versionType: "snapshot", installed: false },
    ]);

    expect(
      filterVersionGroups(groups, "1.20").map((group) => ({
        major: group.major,
        ids: group.versions.map((version) => version.id),
      })),
    ).toEqual([{ major: "1.20", ids: ["1.20.1"] }]);

    expect(filterVersionGroups(groups, "  ").length).toBe(groups.length);
    expect(filterVersionGroups(groups, "nope")).toEqual([]);
  });
});
