import {
  DOWNLOAD_SPEED_ERROR,
  downloadLimitKbs,
  parseDownloadLimit,
} from "@/lib/download-speed";
import type { JavaPaths } from "@/lib/generated/JavaPaths";
import type { Settings } from "@/lib/generated/Settings";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

export type Draft = {
  theme: Settings["theme"];
  username: string;
  fullscreen: boolean;
  width: string;
  height: string;
  maxMemoryMb: number;
  jvmArguments: string;
  java25: string;
  java21: string;
  java17: string;
  java8: string;
  downloadConcurrency: number;
  writeConcurrency: number;
  downloadSpeedLimitKbs: string;
  dataDirectory: string;
  disableCustomTitleBar: boolean;
  automaticUpdates: boolean;
  language: Settings["language"];
  hardwareAcceleration: boolean;
  minimizeToTray: boolean;
  minimizeOnGameStart: boolean;
  restoreOnGameExit: boolean;
};

function installedPath(
  saved: string | null,
  installed: string | null | undefined,
): string {
  if (saved) {
    return saved;
  }
  return installed ?? "";
}

export function fromSnapshot(
  snapshot: SettingsSnapshot,
  runtime: JavaPaths | null,
): Draft {
  const settings = snapshot.settings;
  return {
    theme: settings.theme,
    username: settings.username,
    fullscreen: settings.fullscreen,
    width: settings.width === null ? "" : String(settings.width),
    height: settings.height === null ? "" : String(settings.height),
    maxMemoryMb: Math.min(
      snapshot.memoryMaximumMb,
      Math.max(snapshot.memoryMinimumMb, settings.maxMemoryMb),
    ),
    jvmArguments: settings.jvmArguments.join(" "),
    java25: installedPath(settings.java25, runtime?.java25),
    java21: installedPath(settings.java21, runtime?.java21),
    java17: installedPath(settings.java17, runtime?.java17),
    java8: installedPath(settings.java8, runtime?.java8),
    downloadConcurrency: settings.downloadConcurrency,
    writeConcurrency: settings.writeConcurrency,
    downloadSpeedLimitKbs: downloadLimitKbs(
      settings.downloadSpeedLimitBytesPerSecond,
    ),
    dataDirectory: snapshot.applicationDirectory,
    disableCustomTitleBar: settings.disableCustomTitleBar,
    automaticUpdates: settings.automaticUpdates,
    language: settings.language,
    hardwareAcceleration: settings.hardwareAcceleration,
    minimizeToTray: settings.minimizeToTray,
    minimizeOnGameStart: settings.minimizeOnGameStart,
    restoreOnGameExit: settings.restoreOnGameExit,
  };
}

function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

export function optionalSize(value: string): number | null | "invalid" {
  const trimmed = value.trim();
  if (trimmed === "") {
    return null;
  }
  const parsed = Number(trimmed);
  if (!Number.isInteger(parsed) || parsed < 1) {
    return "invalid";
  }
  return parsed;
}

function splitArguments(value: string): string[] {
  return value.split(/\s+/).filter((part) => part.length > 0);
}

export function toSettings(
  draft: Draft,
  schema: number,
  width: number | null,
  height: number | null,
): Settings {
  const speed = parseDownloadLimit(draft.downloadSpeedLimitKbs);
  if (speed === "invalid") throw new Error(DOWNLOAD_SPEED_ERROR);
  return {
    schema,
    theme: draft.theme,
    username: draft.username.trim(),
    fullscreen: draft.fullscreen,
    width,
    height,
    maxMemoryMb: draft.maxMemoryMb,
    jvmArguments: splitArguments(draft.jvmArguments),
    java25: emptyToNull(draft.java25),
    java21: emptyToNull(draft.java21),
    java17: emptyToNull(draft.java17),
    java8: emptyToNull(draft.java8),
    downloadConcurrency: draft.downloadConcurrency,
    writeConcurrency: draft.writeConcurrency,
    downloadSpeedLimitBytesPerSecond: speed,
    dataDirectory: emptyToNull(draft.dataDirectory),
    disableCustomTitleBar: draft.disableCustomTitleBar,
    automaticUpdates: draft.automaticUpdates,
    language: draft.language,
    hardwareAcceleration: draft.hardwareAcceleration,
    minimizeToTray: draft.minimizeToTray,
    minimizeOnGameStart: draft.minimizeOnGameStart,
    restoreOnGameExit: draft.restoreOnGameExit,
  };
}
