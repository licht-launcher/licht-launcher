import { describe, expect, it } from "vitest";

import {
  describeInstanceVersion,
  formatInstanceVersionLine,
} from "@/features/instances/lib/instance-version";

describe("describeInstanceVersion", () => {
  it("parses fabric and quilt loader ids", () => {
    expect(describeInstanceVersion("fabric-loader-0.16.0-1.20.1")).toEqual({
      gameVersion: "1.20.1",
      loaderLabel: "Fabric 0.16.0",
    });
    expect(describeInstanceVersion("quilt-loader-0.26.0-1.21")).toEqual({
      gameVersion: "1.21",
      loaderLabel: "Quilt 0.26.0",
    });
  });

  it("treats plain ids as vanilla", () => {
    expect(describeInstanceVersion("1.20.1")).toEqual({
      gameVersion: "1.20.1",
      loaderLabel: "Vanilla",
    });
  });

  it("formats a single display line", () => {
    expect(formatInstanceVersionLine("fabric-loader-0.16.0-1.20.1")).toBe(
      "1.20.1 · Fabric 0.16.0",
    );
    expect(formatInstanceVersionLine("1.5.2")).toBe("1.5.2 · Vanilla");
    expect(formatInstanceVersionLine("forge-47.2.0")).toBe("Forge 47.2.0");
  });
});
