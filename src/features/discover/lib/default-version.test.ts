import { describe, expect, it } from "vitest";

import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";

import { defaultVersionId } from "./default-version";

function version(
  id: string,
  versionType: ModpackVersionItem["versionType"],
): ModpackVersionItem {
  return {
    id,
    name: id,
    gameVersions: [],
    loaders: [],
    versionType,
    publishedAt: null,
    downloads: 0,
  };
}

describe("defaultVersionId", () => {
  it("picks the newest release over newer betas and alphas", () => {
    expect(
      defaultVersionId([
        version("v9-alpha", "alpha"),
        version("v8-beta", "beta"),
        version("v7", "release"),
        version("v6", "release"),
      ]),
    ).toBe("v7");
  });

  it("falls back to the newest version when there is no release", () => {
    expect(
      defaultVersionId([version("v2-beta", "beta"), version("v1", "alpha")]),
    ).toBe("v2-beta");
  });

  it("has nothing to pick from an empty list", () => {
    expect(defaultVersionId([])).toBeNull();
  });
});
