import { APP_VERSION } from "@/lib/app-release";
import type { BuildDiagnostics } from "@/lib/generated/BuildDiagnostics";

export function parseBuildDiagnostics(value: unknown): BuildDiagnostics {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("Invalid build diagnostics");
  }
  const data = value as Record<string, unknown>;
  const strings = [
    "hostVersion",
    "tauriVersion",
    "osName",
    "systemArchitecture",
    "processArchitecture",
  ] as const;
  const nullableStrings = [
    "hostRevision",
    "webviewVersion",
    "osVersion",
    "kernelVersion",
  ] as const;
  if (
    typeof data.debug !== "boolean" ||
    strings.some((key) => typeof data[key] !== "string") ||
    nullableStrings.some(
      (key) => data[key] !== null && typeof data[key] !== "string",
    )
  ) {
    throw new Error("Invalid build diagnostics");
  }
  return value as BuildDiagnostics;
}

export type BuildDiagnosticsEntry = { label: string; value: string };

/** Labeled build and system details, in the order they are copied. */
export function buildDiagnosticsEntries(
  data: BuildDiagnostics,
): BuildDiagnosticsEntry[] {
  const [version, target, revision] = buildIdentifier().split(" ");
  const architectureBits: Record<string, number | undefined> = {
    x86_64: 64,
    x64: 64,
    aarch64: 64,
    arm64: 64,
    ia64: 64,
    x86: 32,
    i386: 32,
    i586: 32,
    i686: 32,
    arm: 32,
    armv7: 32,
    armv7l: 32,
  };
  const bits = architectureBits[data.systemArchitecture];
  const os = bits === undefined ? data.osName : `${data.osName} ${bits}-bit`;
  return [
    {
      label: "Build",
      value: `Early Release ${version} ${target} (${revision})`,
    },
    { label: "Profile", value: data.debug ? "Debug" : "Release" },
    { label: "Tauri", value: data.tauriVersion },
    { label: "WebView", value: data.webviewVersion ?? "N/A" },
    { label: "OS", value: `${os} (${data.osVersion ?? "N/A"})` },
  ];
}

export function formatBuildDiagnostics(data: BuildDiagnostics): string {
  return buildDiagnosticsEntries(data)
    .map(({ label, value }) => `${label}: ${value}`)
    .join("\n");
}

export function buildIdentifier(
  platform = import.meta.env.VITE_BUILD_PLATFORM,
  architecture = import.meta.env.VITE_BUILD_ARCHITECTURE,
  revision = import.meta.env.VITE_BUILD_REVISION,
): string {
  const platformLabel = platform === "windows" ? "win" : platform;
  const architectureLabel =
    architecture === "x86_64"
      ? "x64"
      : architecture === "aarch64"
        ? "arm64"
        : architecture;
  return `${APP_VERSION} ${platformLabel}-${architectureLabel} ${revision}`;
}
