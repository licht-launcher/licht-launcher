import { describe, expect, it } from "vitest";

import { globalValues } from "@/features/instance-settings/model/global-values";
import { snapshot } from "@/features/settings/fixtures/snapshot";

describe("globalValues", () => {
  it("describes the global values an instance falls back to", () => {
    const values = globalValues({
      ...snapshot.settings,
      fullscreen: false,
      width: 1280,
      height: 720,
      maxMemoryMb: 4096,
      jvmArguments: ["-XX:+UseG1GC"],
    });
    expect(values.window).toBe("Using global settings: 1280 × 720.");
    expect(values.memory).toBe("Using global settings: 4GB.");
    expect(values.arguments).toBe("Using global settings: -XX:+UseG1GC");
  });

  it("uses the game window size and says when nothing is set", () => {
    const values = globalValues({
      ...snapshot.settings,
      fullscreen: false,
      width: null,
      height: null,
      jvmArguments: [],
    });
    expect(values.window).toBe("Using global settings: 854 × 480.");
    expect(values.arguments).toBe("Using global settings: no extra arguments.");
    expect(
      globalValues({ ...snapshot.settings, fullscreen: true }).window,
    ).toBe("Using global settings: full screen.");
  });
});
