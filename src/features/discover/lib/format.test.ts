import { describe, expect, it } from "vitest";

import {
  displayVersionLabel,
  formatDownloads,
  formatTimeAgo,
  formatUpdatedAt,
  formatVersionLabel,
} from "./format";

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
        loaders: ["neoforge"],
        versionType: "release",
        publishedAt: null,
        downloads: 0,
      }),
    ).toBe("7.0.0 (1.21.10 · NeoForge)");
    expect(
      formatVersionLabel({
        id: "f1",
        name: "ATM9 1.0",
        gameVersions: [],
        loaders: [],
        versionType: "release",
        publishedAt: null,
        downloads: 0,
      }),
    ).toBe("ATM9 1.0");
  });
});

const version = {
  id: "v1",
  versionType: "release" as const,
  publishedAt: null,
  downloads: 0,
};

describe("displayVersionLabel", () => {
  it("keeps a label that fits the select", () => {
    expect(
      displayVersionLabel({
        ...version,
        name: "7.0.0",
        gameVersions: ["1.21.10"],
        loaders: ["neoforge"],
      }),
    ).toBe("7.0.0 (1.21.10 · NeoForge)");
  });

  it("cuts a long version name and keeps the game version and loader", () => {
    expect(
      displayVersionLabel({
        ...version,
        name: "Fresh & Smooth 26.3 RELEASE - BIG Player Animation Update too!",
        gameVersions: ["26.3"],
        loaders: ["fabric"],
      }),
    ).toBe("Fresh & Smooth 26.3 RELEASE... (26.3 · Fabric)");
  });

  it("cuts a long name that has no game version or loader", () => {
    expect(
      displayVersionLabel({
        ...version,
        name: "x".repeat(49),
        gameVersions: [],
        loaders: [],
      }),
    ).toBe(`${"x".repeat(45)}...`);
  });

  it("cuts the whole label when the details alone do not fit", () => {
    expect(
      displayVersionLabel({
        ...version,
        name: "Pack",
        gameVersions: [
          "1.20.1",
          "1.20.2",
          "1.21",
          "1.21.1",
          "1.21.4",
          "1.21.8",
          "1.21.10",
          "1.21.11",
        ],
        loaders: ["fabric", "neoforge"],
      }),
    ).toBe("Pack (1.20.1, 1.20.2, 1.21, 1.21.1, 1.21.4, 1...");
  });
});

describe("formatTimeAgo", () => {
  const now = Date.parse("2026-11-10T12:00:00Z");

  it("names the largest whole unit that has passed", () => {
    expect(formatTimeAgo("2026-11-10T11:59:30Z", now)).toBe("just now");
    expect(formatTimeAgo("2026-11-10T11:15:00Z", now)).toBe("45 minutes ago");
    expect(formatTimeAgo("2026-11-10T09:00:00Z", now)).toBe("3 hours ago");
    expect(formatTimeAgo("2026-11-09T10:00:00Z", now)).toBe("yesterday");
    expect(formatTimeAgo("2026-11-01T10:43:19.109719+00:00", now)).toBe(
      "last week",
    );
    expect(formatTimeAgo("2026-08-01T00:00:00Z", now)).toBe("3 months ago");
    expect(formatTimeAgo("2025-04-02T07:44:02.187Z", now)).toBe("last year");
  });

  it("counts whole units for a date slightly ahead of the clock", () => {
    expect(formatTimeAgo("2026-11-10T13:30:00Z", now)).toBe("in 1 hour");
  });

  it("returns null when there is no readable date", () => {
    expect(formatTimeAgo(null, now)).toBeNull();
    expect(formatTimeAgo("soon", now)).toBeNull();
  });
});
