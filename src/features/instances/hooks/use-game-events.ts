import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import type { ConsoleLine } from "@/features/instances/model/console-line";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { GameLogEvent } from "@/lib/generated/GameLogEvent";
import type { InstallProgress } from "@/lib/generated/InstallProgress";

function toDownloadProgress(progress: InstallProgress): DownloadProgress {
  return {
    finished: progress.finished,
    failed: progress.failed,
    total: progress.total,
    bytesDone: progress.bytesDone,
    bytesTotal: progress.bytesTotal,
  };
}

export function useGameEvents(onGameLog?: (folder: string) => void) {
  const [progressByVersion, setProgressByVersion] = useState<
    Record<string, DownloadProgress>
  >({});
  const [logsByFolder, setLogsByFolder] = useState<
    Record<string, ConsoleLine[]>
  >({});
  const [speedByVersion, setSpeedByVersion] = useState<Record<string, number>>(
    {},
  );
  const previousProgress = useRef<
    Record<string, { bytes: number; timestamp: number }>
  >({});
  const nextLine = useRef(0);
  const onGameLogRef = useRef(onGameLog);
  onGameLogRef.current = onGameLog;

  useEffect(() => {
    let stop = false;
    let unlistenProgress: (() => void) | undefined;
    let unlistenLog: (() => void) | undefined;
    void listen<InstallProgress>("install-progress", (event) => {
      const { versionId } = event.payload;
      const now = Date.now();
      const previous = previousProgress.current[versionId];
      if (previous !== undefined) {
        const elapsed = now - previous.timestamp;
        if (elapsed > 0) {
          setSpeedByVersion((current) => ({
            ...current,
            [versionId]: Math.max(
              0,
              Math.round(
                ((event.payload.bytesDone - previous.bytes) * 1000) / elapsed,
              ),
            ),
          }));
        }
      }
      previousProgress.current[versionId] = {
        bytes: event.payload.bytesDone,
        timestamp: now,
      };
      setProgressByVersion((current) => ({
        ...current,
        [versionId]: toDownloadProgress(event.payload),
      }));
    }).then((unlisten) => {
      if (stop) {
        unlisten();
      } else {
        unlistenProgress = unlisten;
      }
    });
    void listen<GameLogEvent>("game-log", (event) => {
      const id = nextLine.current;
      nextLine.current += 1;
      const { folder, stream, line } = event.payload;
      onGameLogRef.current?.(folder);
      setLogsByFolder((current) => ({
        ...current,
        [folder]: [...(current[folder] ?? []), { id, stream, line }],
      }));
    }).then((unlisten) => {
      if (stop) {
        unlisten();
      } else {
        unlistenLog = unlisten;
      }
    });
    return () => {
      stop = true;
      unlistenProgress?.();
      unlistenLog?.();
    };
  }, []);

  function clearFolderLog(folder: string) {
    setLogsByFolder((current) => ({ ...current, [folder]: [] }));
  }

  function clearVersionProgress(versionId: string) {
    setProgressByVersion((current) => {
      if (!(versionId in current)) {
        return current;
      }
      const next = { ...current };
      delete next[versionId];
      return next;
    });
    setSpeedByVersion((current) => {
      if (!(versionId in current)) {
        return current;
      }
      const next = { ...current };
      delete next[versionId];
      return next;
    });
    delete previousProgress.current[versionId];
  }

  function progressForVersion(versionId: string | null | undefined) {
    if (versionId === null || versionId === undefined) {
      return null;
    }
    return progressByVersion[versionId] ?? null;
  }

  return {
    progressForVersion,
    speedForVersion: (versionId: string | null | undefined) =>
      versionId === null || versionId === undefined
        ? 0
        : (speedByVersion[versionId] ?? 0),
    clearVersionProgress,
    logsByFolder,
    clearFolderLog,
  };
}
