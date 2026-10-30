import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import {
  getUpdateSimulation,
  subscribeUpdateSimulation,
} from "@/features/updater/model/simulation";
import { APP_VERSION } from "@/lib/app-release";
import { parseAppUpdateState } from "@/lib/app-update-state";
import { getAppUpdateState } from "@/lib/commands";
import { messageOf } from "@/lib/errors";
import type { AppUpdateState } from "@/lib/generated/AppUpdateState";

export const APP_UPDATE_QUERY_KEY = ["app-update-state"] as const;

export function useAppUpdateState() {
  const client = useQueryClient();
  const [ready, setReady] = useState(false);
  const [eventError, setEventError] = useState<string | null>(null);
  const sequence = useRef(0);
  const latest = useRef<AppUpdateState | null>(null);
  const simulated = useSyncExternalStore(
    subscribeUpdateSimulation,
    getUpdateSimulation,
    () => false,
  );
  useEffect(() => {
    let stopped = false;
    let unsubscribe: (() => void) | undefined;
    void listen<unknown>("app-update-state", ({ payload }) => {
      if (stopped) return;
      try {
        const state = parseAppUpdateState(payload);
        sequence.current++;
        latest.current = state;
        client.setQueryData(APP_UPDATE_QUERY_KEY, state);
      } catch (error: unknown) {
        setEventError(messageOf(error));
      }
    })
      .then((off) => {
        if (stopped) {
          off();
          return;
        }
        unsubscribe = off;
        setReady(true);
      })
      .catch((error: unknown) => {
        if (stopped) return;
        setEventError(messageOf(error));
        setReady(true);
      });
    return () => {
      stopped = true;
      unsubscribe?.();
    };
  }, [client]);
  const query = useQuery({
    queryKey: APP_UPDATE_QUERY_KEY,
    enabled: ready,
    retry: false,
    refetchInterval: eventError ? 500 : false,
    queryFn: async () => {
      const before = sequence.current;
      const state = await getAppUpdateState();
      return sequence.current !== before && latest.current
        ? latest.current
        : state;
    },
  });
  const state: AppUpdateState | undefined = simulated
    ? {
        phase: "ready",
        automaticUpdates: query.data?.automaticUpdates ?? false,
        version: `${APP_VERSION}-dev`,
        downloadedBytes: 0,
        totalBytes: null,
        error: null,
      }
    : query.data;
  return { ...query, data: state, simulated, eventError };
}
