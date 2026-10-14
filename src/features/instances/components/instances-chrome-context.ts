import type { UseQueryResult } from "@tanstack/react-query";
import { createContext, useContext } from "react";

import type { useGameEvents } from "@/features/instances/hooks/use-game-events";
import type {
  PlayPhase,
  usePlaySession,
} from "@/features/instances/hooks/use-play-session";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export type InstancesChromeContext = {
  instances: UseQueryResult<InstanceEntry[], Error>;
  username: string;
  selectedFolder: string | null;
  setSelectedFolder: (folder: string | null) => void;
  selected: InstanceEntry | null;
  busy: boolean;
  playBlocked: boolean;
  playPhase: (folder: string) => PlayPhase;
  isInstalling: (folder: string) => boolean;
  progressForVersion: ReturnType<typeof useGameEvents>["progressForVersion"];
  speedForVersion: ReturnType<typeof useGameEvents>["speedForVersion"];
  logsByFolder: ReturnType<typeof useGameEvents>["logsByFolder"];
  clearFolderLog: ReturnType<typeof useGameEvents>["clearFolderLog"];
  lastPlayedFolder: string | null;
  playInstance: ReturnType<typeof usePlaySession>["playInstance"];
  prepareInstance: ReturnType<typeof usePlaySession>["prepareInstance"];
  stopInstance: ReturnType<typeof usePlaySession>["stopInstance"];
  openConfigure: (folder: string) => void;
};

export const ChromeContext = createContext<InstancesChromeContext | null>(null);

export function useInstancesChrome(): InstancesChromeContext {
  const value = useContext(ChromeContext);
  if (value === null) {
    throw new Error("useInstancesChrome must be used inside InstancesChrome");
  }
  return value;
}
