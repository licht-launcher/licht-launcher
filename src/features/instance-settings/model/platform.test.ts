import { describe, expect, it } from "vitest";

import { formatPlatform } from "./platform";

describe("formatPlatform", () => {
  it("names the loader with its build", () => {
    expect(
      formatPlatform({
        platform: "neoForge",
        gameVersion: "1.21.1",
        loaderVersion: "21.1.77",
      }),
    ).toBe("NeoForge 21.1.77");
  });

  it("names Vanilla without a build", () => {
    expect(
      formatPlatform({
        platform: "vanilla",
        gameVersion: "1.21.1",
        loaderVersion: null,
      }),
    ).toBe("Vanilla");
  });

  it("says Unknown when the version has no readable profile", () => {
    expect(formatPlatform(null)).toBe("Unknown");
  });
});
