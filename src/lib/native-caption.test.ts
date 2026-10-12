import { afterEach, describe, expect, it, vi } from "vitest";

import {
  NATIVE_CAPTION_WIDTH_PX,
  usesNativeCaptionButtons,
} from "@/lib/native-caption";

describe("native-caption", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  });

  it("reserves space for three 46px caption buttons", () => {
    expect(NATIVE_CAPTION_WIDTH_PX).toBe(138);
  });

  it("is off outside Tauri", () => {
    vi.stubGlobal("navigator", { userAgent: "Windows NT 10.0" });
    expect(usesNativeCaptionButtons()).toBe(false);
  });

  it("is on for Windows Tauri", () => {
    vi.stubGlobal("navigator", { userAgent: "Windows NT 10.0; Win64" });
    Object.assign(window, { __TAURI_INTERNALS__: {} });
    expect(usesNativeCaptionButtons()).toBe(true);
  });

  it("is off for Linux Tauri", () => {
    vi.stubGlobal("navigator", { userAgent: "X11; Linux x86_64" });
    Object.assign(window, { __TAURI_INTERNALS__: {} });
    expect(usesNativeCaptionButtons()).toBe(false);
  });
});
