import type { AppUpdatePhase } from "@/lib/generated/AppUpdatePhase";
import type { AppUpdateState } from "@/lib/generated/AppUpdateState";

const phases: Record<AppUpdatePhase, true> = {
  idle: true,
  checking: true,
  upToDate: true,
  available: true,
  downloading: true,
  verifying: true,
  cached: true,
  ready: true,
  installing: true,
  failed: true,
};

function byteCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

export function parseAppUpdateState(value: unknown): AppUpdateState {
  if (
    typeof value !== "object" ||
    value === null ||
    !("phase" in value) ||
    typeof value.phase !== "string" ||
    !Object.hasOwn(phases, value.phase) ||
    !("automaticUpdates" in value) ||
    typeof value.automaticUpdates !== "boolean" ||
    !("version" in value) ||
    !(value.version === null || typeof value.version === "string") ||
    !("downloadedBytes" in value) ||
    !byteCount(value.downloadedBytes) ||
    !("totalBytes" in value) ||
    !(value.totalBytes === null || byteCount(value.totalBytes)) ||
    !("error" in value) ||
    !(value.error === null || typeof value.error === "string")
  ) {
    throw new Error("Invalid application update state");
  }
  return value as AppUpdateState;
}
