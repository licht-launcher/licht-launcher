import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { buildDiagnostics } from "@/features/settings/fixtures/build-diagnostics";
import { APP_VERSION } from "@/lib/app-release";
import {
  buildIdentifier,
  formatBuildDiagnostics,
  parseBuildDiagnostics,
} from "@/lib/build-info";

describe("buildIdentifier", () => {
  it.each([
    ["windows", "x86_64", "win-x64"],
    ["linux", "aarch64", "linux-arm64"],
    ["linux", "x64", "linux-x64"],
  ])(
    "includes the version, %s target and commit revision",
    (platform, architecture, expected) => {
      expect(buildIdentifier(platform, architecture, "abc1234")).toBe(
        `${APP_VERSION} ${expected} abc1234`,
      );
    },
  );
});

describe("build diagnostics", () => {
  beforeEach(() => {
    vi.stubEnv("VITE_BUILD_PLATFORM", "windows");
    vi.stubEnv("VITE_BUILD_ARCHITECTURE", "x86_64");
    vi.stubEnv("VITE_BUILD_REVISION", "a1aa4a6");
  });
  afterEach(() => vi.unstubAllEnvs());

  it("copies the concise five-line early-release diagnostics", () => {
    expect(formatBuildDiagnostics({ ...buildDiagnostics, debug: true })).toBe(
      [
        `Build: Early Release ${APP_VERSION} win-x64 (a1aa4a6)`,
        "Profile: Debug",
        "Tauri: 2.12.1",
        "WebView: 156.0.4314.50",
        "OS: Windows 11 Pro 64-bit (10.0.26300)",
      ].join("\n"),
    );
  });

  it("uses system bitness for Linux even when the process architecture differs", () => {
    vi.stubEnv("VITE_BUILD_PLATFORM", "linux");
    vi.stubEnv("VITE_BUILD_ARCHITECTURE", "aarch64");
    const text = formatBuildDiagnostics({
      ...buildDiagnostics,
      debug: true,
      osName: "Ubuntu 24.04",
      osVersion: "24.04",
      kernelVersion: "6.8.0",
      systemArchitecture: "aarch64",
      processArchitecture: "arm",
    });
    expect(text).toContain("Profile: Debug");
    expect(text).toContain(
      `Build: Early Release ${APP_VERSION} linux-arm64 (a1aa4a6)`,
    );
    expect(text).toContain("OS: Ubuntu 24.04 64-bit (24.04)");
    expect(text.split("\n")).toHaveLength(5);
  });

  it("keeps the release profile and identifies a 32-bit system", () => {
    const text = formatBuildDiagnostics({
      ...buildDiagnostics,
      osName: "Windows 10 Pro",
      osVersion: "10.0.19045",
      systemArchitecture: "x86",
      processArchitecture: "x86",
    });
    expect(text).toContain("Profile: Release");
    expect(text).toContain("OS: Windows 10 Pro 32-bit (10.0.19045)");
  });

  it("does not guess bitness for an unknown system architecture", () => {
    expect(
      formatBuildDiagnostics({
        ...buildDiagnostics,
        systemArchitecture: "unknown",
      }),
    ).toContain("OS: Windows 11 Pro (10.0.26300)");
  });

  it("labels unavailable data without guessing versions", () => {
    const text = formatBuildDiagnostics({
      ...buildDiagnostics,
      hostRevision: null,
      webviewVersion: null,
      osVersion: null,
      kernelVersion: null,
    });
    expect(text).toContain("WebView: N/A");
    expect(text).toContain("OS: Windows 11 Pro 64-bit (N/A)");
  });

  it("validates the native diagnostics response", () => {
    expect(parseBuildDiagnostics(buildDiagnostics)).toEqual(buildDiagnostics);
    expect(
      parseBuildDiagnostics({
        ...buildDiagnostics,
        hostRevision: null,
        webviewVersion: null,
        osVersion: null,
        kernelVersion: null,
      }),
    ).toMatchObject({ webviewVersion: null });
  });

  it.each([
    null,
    [],
    {},
    { ...buildDiagnostics, debug: "false" },
    { ...buildDiagnostics, hostVersion: 3 },
    { ...buildDiagnostics, webviewVersion: 128 },
    { ...buildDiagnostics, processArchitecture: null },
  ])("rejects malformed native diagnostics: %j", (value) => {
    expect(() => parseBuildDiagnostics(value)).toThrow(
      "Invalid build diagnostics",
    );
  });
});
