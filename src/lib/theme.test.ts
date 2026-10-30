import { afterEach, describe, expect, it, vi } from "vitest";
import { initialTheme, isLauncherTheme, resolveTheme } from "@/lib/theme";

describe("launcher themes", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("accepts only supported native theme values", () => {
    for (const value of ["light", "dark", "system"])
      expect(isLauncherTheme(value)).toBe(true);
    for (const value of [null, undefined, {}, "unknown", "Dark", 1])
      expect(isLauncherTheme(value)).toBe(false);
  });

  it("resolves system preferences while keeping explicit choices independent of the OS", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("uses the native startup preference and falls back to dark for invalid bootstrap data", () => {
    vi.stubGlobal("__LICHT_THEME__", "system");
    expect(initialTheme()).toBe("system");
    vi.stubGlobal("__LICHT_THEME__", "invalid");
    expect(initialTheme()).toBe("dark");
  });
});
