import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { VERSIONS_QUERY_KEY } from "@/features/instances/lib/versions-query";
import {
  installModpackFiles,
  installVersion,
  isVersionReady,
  launchVersion,
  listRunningGames,
  stopGame,
  touchLastOpened,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export type PlayPhase = "idle" | "starting" | "running" | "stopping";

export function usePlaySession({
  username,
  clearVersionProgress,
  clearFolderLog,
}: {
  username: string;
  clearVersionProgress: (versionId: string) => void;
  clearFolderLog: (folder: string) => void;
}) {
  const queryClient = useQueryClient();
  /** folder → versionId while that instance is installing. */
  const [installingByFolder, setInstallingByFolder] = useState<
    Record<string, string>
  >({});
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
  const installingRef = useRef(new Map<string, string>());
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

  function beginInstall(folder: string, versionId: string) {
    installingRef.current.set(folder, versionId);
    setInstallingByFolder((current) => ({ ...current, [folder]: versionId }));
  }

  function endInstall(folder: string, versionId: string) {
    installingRef.current.delete(folder);
    setInstallingByFolder((current) => {
      const next = { ...current };
      delete next[folder];
      return next;
    });
    const stillUsingVersion = [...installingRef.current.values()].includes(
      versionId,
    );
    if (!stillUsingVersion) {
      clearVersionProgress(versionId);
    }
  }

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

  /** Downloads game/loader assets and modpack files; does not launch. */
  async function prepareInstance(instance: InstanceEntry) {
    if (
      sessionRef.current.has(instance.folder) ||
      installingRef.current.has(instance.folder)
    ) {
      return;
    }

    const ready = await isVersionReady(instance.versionId);
    if (ready) {
      beginInstall(instance.folder, instance.versionId);
      try {
        await installModpackFiles(instance.folder);
      } finally {
        endInstall(instance.folder, instance.versionId);
      }
      return;
    }

    beginInstall(instance.folder, instance.versionId);
    try {
      await installVersion(instance.versionId);
      await queryClient.invalidateQueries({ queryKey: VERSIONS_QUERY_KEY });
      await installModpackFiles(instance.folder);
    } finally {
      endInstall(instance.folder, instance.versionId);
    }
  }

  async function playInstance(instance: InstanceEntry) {
    if (
      playBlocked ||
      sessionRef.current.has(instance.folder) ||
      installingRef.current.has(instance.folder)
    ) {
      return;
    }
    clearFolderLog(instance.folder);

    try {
      const ready = await isVersionReady(instance.versionId);
      if (!ready) {
        await prepareInstance(instance);
      }
      markStarting(instance.folder);
      launchPendingRef.current.add(instance.folder);
      try {
        await touchLastOpened(instance.folder);
        void queryClient.invalidateQueries({ queryKey: ["last-played"] });
        void queryClient.invalidateQueries({ queryKey: ["instances"] });
        await launchVersion(instance.folder, username);
      } finally {
        clearSession(instance.folder);
      }
    } catch (error: unknown) {
      if (installingRef.current.has(instance.folder)) {
        endInstall(instance.folder, instance.versionId);
      }
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

  function isInstalling(folder: string) {
    return folder in installingByFolder;
  }

  return {
    installingByFolder,
    isInstalling,
    beginInstall,
    endInstall,
    playBlocked,
    playPhase,
    markStarted,
    prepareInstance,
    playInstance,
    stopInstance,
  };
}
