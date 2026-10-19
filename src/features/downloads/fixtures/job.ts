import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";

export function downloadJob(
  overrides: Partial<DownloadJobSnapshot> = {},
): DownloadJobSnapshot {
  return {
    id: "1",
    instanceFolder: "Survival",
    versionId: "1.20.1",
    operation: "install",
    status: "running",
    component: "minecraft",
    phase: "downloading",
    progress: {
      finished: 1,
      failed: 0,
      total: 4,
      bytesDone: 100_000,
      bytesTotal: 400_000,
    },
    createdAt: 1_700_000_000_000,
    updatedAt: 1_700_000_001_000,
    ...overrides,
  };
}

export function completedDownload(
  overrides: Partial<DownloadJobSnapshot> = {},
): DownloadJobSnapshot {
  return downloadJob({
    status: "completed",
    progress: {
      finished: 4,
      failed: 0,
      total: 4,
      bytesDone: 400_000,
      bytesTotal: 400_000,
    },
    ...overrides,
  });
}
