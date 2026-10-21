import type { StartupProgress } from "@/lib/generated/StartupProgress";

export function startupProgress(
  overrides: Partial<StartupProgress> = {},
): StartupProgress {
  return {
    phase: "ready",
    processedFiles: 0,
    processedBytes: 0,
    totalBytes: null,
    version: null,
    updateStatus: "notChecked",
    warning: null,
    error: null,
    preview: false,
    ...overrides,
  };
}
