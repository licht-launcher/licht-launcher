import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { getStartupProgress } from "@/lib/commands";
import type { StartupProgress } from "@/lib/generated/StartupProgress";

const key = ["startup-progress"] as const;

export function useStartupProgress(ignorePreview = false) {
  const client = useQueryClient();
  const [ready, setReady] = useState(false);
  const [eventError, setEventError] = useState<string | null>(null);
  const latest = useRef<StartupProgress | null>(null);
  const sequence = useRef(0);
  useEffect(() => {
    let stopped = false;
    let unsubscribe: (() => void) | undefined;
    void listen<StartupProgress>("startup-progress", ({ payload }) => {
      if (stopped || (ignorePreview && payload.preview)) return;
      sequence.current++;
      latest.current = payload;
      client.setQueryData(key, payload);
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
        setEventError(error instanceof Error ? error.message : String(error));
        setReady(true);
      });
    return () => {
      stopped = true;
      unsubscribe?.();
    };
  }, [client, ignorePreview]);
  const query = useQuery({
    queryKey: key,
    enabled: ready,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
    refetchInterval: (query) =>
      eventError &&
      query.state.data?.phase !== "ready" &&
      query.state.data?.phase !== "failed"
        ? 500
        : false,
    queryFn: async () => {
      const before = sequence.current;
      const snapshot = await getStartupProgress();
      return sequence.current !== before && latest.current
        ? latest.current
        : snapshot;
    },
  });
  return { ...query, eventError };
}
