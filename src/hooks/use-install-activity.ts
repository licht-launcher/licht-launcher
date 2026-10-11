import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { progressText } from "@/features/instances/lib/download-progress";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstallProgress } from "@/lib/generated/InstallProgress";

/** True while files are still transferring. A 0/0 pulse is not a lasting install. */
export function isInstallProgressActive(progress: InstallProgress): boolean {
  if (progress.total === 0) {
    return false;
  }
  return progress.finished + progress.failed < progress.total;
}

function toDownloadProgress(progress: InstallProgress): DownloadProgress {
  return {
    finished: progress.finished,
    failed: progress.failed,
    total: progress.total,
    bytesDone: progress.bytesDone,
    bytesTotal: progress.bytesTotal,
  };
}

/** Tracks install-progress for chrome that is outside the instances page. */
export function useInstallActivity() {
  const [activeByVersion, setActiveByVersion] = useState<
    Record<string, InstallProgress>
  >({});

  useEffect(() => {
    let stop = false;
    let unlisten: (() => void) | undefined;
    void listen<InstallProgress>("install-progress", (event) => {
      const progress = event.payload;
      setActiveByVersion((current) => {
        if (isInstallProgressActive(progress)) {
          return { ...current, [progress.versionId]: progress };
        }
        if (!(progress.versionId in current)) {
          return current;
        }
        const next = { ...current };
        delete next[progress.versionId];
        return next;
      });
    }).then((next) => {
      if (stop) {
        next();
      } else {
        unlisten = next;
      }
    });
    return () => {
      stop = true;
      unlisten?.();
    };
  }, []);

  const activeEntries = Object.values(activeByVersion);
  const activeCount = activeEntries.length;
  const single = activeCount === 1 ? activeEntries[0] : undefined;
  const progress = single === undefined ? null : toDownloadProgress(single);

  return {
    active: activeCount > 0,
    activeCount,
    progress,
    label:
      activeCount === 0
        ? null
        : single === undefined
          ? `${activeCount} installing`
          : progressText(toDownloadProgress(single)),
  };
}
