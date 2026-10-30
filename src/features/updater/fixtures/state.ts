import type { AppUpdateState } from "@/lib/generated/AppUpdateState";

export function appUpdateState(
  overrides: Partial<AppUpdateState> = {},
): AppUpdateState {
  return {
    phase: "idle",
    automaticUpdates: true,
    version: null,
    downloadedBytes: 0,
    totalBytes: null,
    error: null,
    ...overrides,
  };
}
