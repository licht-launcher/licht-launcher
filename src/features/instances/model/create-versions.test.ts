import { describe, expect, it } from "vitest";

import { createVersionChoices } from "@/features/instances/model/create-versions";
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
