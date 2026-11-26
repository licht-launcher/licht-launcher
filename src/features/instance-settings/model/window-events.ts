import type { InstanceBaseInput } from "@/api/commands";
import type {
  LoaderChannel,
  PlatformId,
} from "@/features/new-instance/model/create-state";

/** An instance settings window asks the launcher window to run an action. */
export const INSTANCE_SETTINGS_ACTION_EVENT = "instance-settings-action";
/** The launcher window reports an action it could not run. */
export const INSTANCE_SETTINGS_ACTION_FAILED_EVENT =
  "instance-settings-action-failed";
/** The launcher window reports what blocks instance actions. */
export const INSTANCE_SETTINGS_STATUS_EVENT = "instance-settings-status";
/** An instance settings window asks for the status once it listens for it. */
export const INSTANCE_SETTINGS_STATUS_REQUEST_EVENT =
  "instance-settings-status-request";

export type InstanceSettingsAction =
  | { kind: "rename"; folder: string; name: string }
  | { kind: "changeBase"; folder: string; base: InstanceBaseInput }
  | { kind: "openFolder"; folder: string }
  | { kind: "duplicate"; folder: string }
  | { kind: "delete"; folder: string };

export type InstanceSettingsStatus = {
  busy: boolean;
  versionBusy: boolean;
  /** Instances whose game or a download for them is running. */
  activeFolders: string[];
};

export type InstanceSettingsActionFailure = {
  folder: string;
  message: string;
};

const PLATFORMS: readonly PlatformId[] = [
  "vanilla",
  "fabric",
  "neoforge",
  "forge",
  "quilt",
];
const LOADER_CHANNELS: readonly LoaderChannel[] = [
  "stable",
  "latest",
  "manual",
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isText(value: unknown): value is string {
  return typeof value === "string" && value !== "";
}

function parseBase(value: unknown): InstanceBaseInput | null {
  if (!isRecord(value)) return null;
  const { platform, gameVersion, loaderChannel, loaderVersion } = value;
  const knownPlatform = PLATFORMS.find((id) => id === platform);
  const knownChannel = LOADER_CHANNELS.find((id) => id === loaderChannel);
  if (knownPlatform === undefined || knownChannel === undefined) return null;
  if (!isText(gameVersion)) return null;
  if (loaderVersion !== null && typeof loaderVersion !== "string") return null;
  return {
    platform: knownPlatform,
    gameVersion,
    loaderChannel: knownChannel,
    loaderVersion,
  };
}

export function parseInstanceSettingsAction(
  value: unknown,
): InstanceSettingsAction | null {
  if (!isRecord(value) || !isText(value.folder)) return null;
  const folder = value.folder;
  switch (value.kind) {
    case "rename":
      return isText(value.name)
        ? { kind: "rename", folder, name: value.name }
        : null;
    case "changeBase": {
      const base = parseBase(value.base);
      return base ? { kind: "changeBase", folder, base } : null;
    }
    case "openFolder":
    case "duplicate":
    case "delete":
      return { kind: value.kind, folder };
    default:
      return null;
  }
}

export function parseInstanceSettingsStatus(
  value: unknown,
): InstanceSettingsStatus | null {
  if (!isRecord(value)) return null;
  const { busy, versionBusy, activeFolders } = value;
  if (typeof busy !== "boolean" || typeof versionBusy !== "boolean")
    return null;
  if (!Array.isArray(activeFolders)) return null;
  const folders = activeFolders.filter(
    (folder: unknown): folder is string => typeof folder === "string",
  );
  if (folders.length !== activeFolders.length) return null;
  return { busy, versionBusy, activeFolders: folders };
}

export function parseInstanceSettingsActionFailure(
  value: unknown,
): InstanceSettingsActionFailure | null {
  if (!isRecord(value)) return null;
  const { folder, message } = value;
  if (!isText(folder) || typeof message !== "string") return null;
  return { folder, message };
}
