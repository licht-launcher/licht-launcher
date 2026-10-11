import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import type { ConsoleLine } from "@/features/instances/model/console-line";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { GameLogEvent } from "@/lib/generated/GameLogEvent";

export function useGameEvents(onGameLog?: (folder: string) => void) {
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [logsByFolder, setLogsByFolder] = useState<
    Record<string, ConsoleLine[]>
  >({});
  const nextLine = useRef(0);
  const onGameLogRef = useRef(onGameLog);
  onGameLogRef.current = onGameLog;

  useEffect(() => {
    let stop = false;
    let unlistenProgress: (() => void) | undefined;
    let unlistenLog: (() => void) | undefined;
    void listen<DownloadProgress>("install-progress", (event) => {
      setProgress(event.payload);
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

  return {
    progress,
    setProgress,
    logsByFolder,
    clearFolderLog,
  };
}
