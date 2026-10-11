import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import {
  installVersion,
  launchVersion,
  listRunningGames,
  stopGame,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

export type PlayPhase = "idle" | "starting" | "running" | "stopping";

export function usePlaySession({
  versions,
  username,
  setProgress,
  clearFolderLog,
}: {
  versions: VersionSummary[] | undefined;
  username: string;
  setProgress: (progress: DownloadProgress | null) => void;
  clearFolderLog: (folder: string) => void;
}) {
  const queryClient = useQueryClient();
  const [installingFolder, setInstallingFolder] = useState<string | null>(null);
  const [startingFolders, setStartingFolders] = useState<Record<string, true>>(
    {},
  );
  const [runningFolders, setRunningFolders] = useState<Record<string, true>>(
    {},
  );
  const [stoppingFolders, setStoppingFolders] = useState<Record<string, true>>(
    {},
  );
  const sessionRef = useRef(new Set<string>());
  const launchPendingRef = useRef(new Set<string>());
  const startingRef = useRef(new Set<string>());
  const playBlocked = username.trim() === "";

  useEffect(() => {
    let cancelled = false;
    void listRunningGames()
      .then((folders) => {
        if (cancelled) {
          return;
        }
        const next: Record<string, true> = {};
        for (const folder of folders) {
          sessionRef.current.add(folder);
          next[folder] = true;
        }
        setRunningFolders((current) => ({ ...current, ...next }));
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          toast.error(friendlyError(messageOf(error)));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  function markStarting(folder: string) {
    sessionRef.current.add(folder);
    startingRef.current.add(folder);
    setStartingFolders((current) => ({ ...current, [folder]: true }));
  }

  function markRunning(folder: string) {
    sessionRef.current.add(folder);
    startingRef.current.delete(folder);
    setStartingFolders((current) => {
      const next = { ...current };
      delete next[folder];
      return next;
    });
    setRunningFolders((current) => ({ ...current, [folder]: true }));
  }

  function markStopping(folder: string) {
    setStoppingFolders((current) => ({ ...current, [folder]: true }));
  }

  function clearSession(folder: string) {
    sessionRef.current.delete(folder);
    startingRef.current.delete(folder);
    launchPendingRef.current.delete(folder);
    setStartingFolders((current) => {
      const next = { ...current };
      delete next[folder];
      return next;
    });
    setRunningFolders((current) => {
      const next = { ...current };
      delete next[folder];
      return next;
    });
    setStoppingFolders((current) => {
      const next = { ...current };
      delete next[folder];
      return next;
    });
  }

  /** First console line means the JVM is up — leave the starting phase. */
  function markStarted(folder: string) {
    if (!startingRef.current.has(folder)) {
      return;
    }
    markRunning(folder);
  }

  async function playInstance(instance: InstanceEntry) {
    if (
      playBlocked ||
      sessionRef.current.has(instance.folder) ||
      installingFolder === instance.folder
    ) {
      return;
    }
    clearFolderLog(instance.folder);

    try {
      const summary = versions?.find(
        (version) => version.id === instance.versionId,
      );
      if (summary === undefined || !summary.installed) {
        setInstallingFolder(instance.folder);
        setProgress(null);
        try {
          await installVersion(instance.versionId);
          await queryClient.invalidateQueries({ queryKey: ["versions"] });
        } finally {
          setInstallingFolder(null);
          setProgress(null);
        }
      }
      markStarting(instance.folder);
      launchPendingRef.current.add(instance.folder);
      try {
        await launchVersion(instance.folder, username);
      } finally {
        clearSession(instance.folder);
      }
    } catch (error: unknown) {
      setInstallingFolder(null);
      setProgress(null);
      clearSession(instance.folder);
      toast.error(friendlyError(messageOf(error)));
    }
  }

  async function stopInstance(folder: string) {
    if (stoppingFolders[folder]) {
      return;
    }
    markStopping(folder);
    try {
      await stopGame(folder);
      if (!launchPendingRef.current.has(folder)) {
        clearSession(folder);
      }
    } catch (error: unknown) {
      setStoppingFolders((current) => {
        const next = { ...current };
        delete next[folder];
        return next;
      });
      toast.error(friendlyError(messageOf(error)));
    }
  }

  function playPhase(folder: string): PlayPhase {
    if (stoppingFolders[folder]) {
      return "stopping";
    }
    if (startingFolders[folder]) {
      return "starting";
    }
    if (runningFolders[folder]) {
      return "running";
    }
    return "idle";
  }

  return {
    installingFolder,
    playBlocked,
    playPhase,
    markStarted,
    playInstance,
    stopInstance,
  };
}
