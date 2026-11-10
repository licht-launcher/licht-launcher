import { describe, expect, it } from "vitest";

import { formatDownloads, formatUpdatedAt, formatVersionLabel } from "./format";

describe("formatDownloads", () => {
  it("abbreviates large counts", () => {
    expect(formatDownloads(950)).toBe("950");
    expect(formatDownloads(12_345)).toBe("12.3K");
    expect(formatDownloads(19_192_625)).toBe("19.2M");
  });
});

describe("formatUpdatedAt", () => {
  it("formats a timestamp as a date", () => {
    expect(formatUpdatedAt("2026-11-01T10:43:19.109719Z")).toBe("Nov 1, 2026");
  });

  it("returns null when there is no readable date", () => {
    expect(formatUpdatedAt(null)).toBeNull();
    expect(formatUpdatedAt("yesterday")).toBeNull();
  });
});

describe("formatVersionLabel", () => {
  it("adds game versions and loaders when the source lists them", () => {
    expect(
      formatVersionLabel({
        id: "v7",
        name: "7.0.0",
        gameVersions: ["1.21.10"],
        loaders: ["fabric"],
      }),
    ).toBe("7.0.0 (1.21.10 · fabric)");
    expect(
      formatVersionLabel({
        id: "f1",
        name: "ATM9 1.0",
        gameVersions: [],
        loaders: [],
      }),
    ).toBe("ATM9 1.0");
  });
});
