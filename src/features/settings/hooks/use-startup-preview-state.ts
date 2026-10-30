import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { isStartupPreviewOpen } from "@/lib/commands";
import { messageOf } from "@/lib/errors";

export const STARTUP_PREVIEW_QUERY_KEY = [
  "dev",
  "startup-preview-open",
] as const;

export function useStartupPreviewState() {
  const client = useQueryClient();
  const [ready, setReady] = useState(false);
  const [eventError, setEventError] = useState<string | null>(null);
  const sequence = useRef(0);
  const latest = useRef(false);
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    let stopped = false;
    let unsubscribe: (() => void) | undefined;
    void listen<unknown>("startup-preview-state", ({ payload }) => {
      if (stopped) return;
      if (typeof payload !== "boolean") {
        setEventError("Invalid startup preview window state");
        return;
      }
      sequence.current++;
      latest.current = payload;
      client.setQueryData(STARTUP_PREVIEW_QUERY_KEY, payload);
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
    queryKey: STARTUP_PREVIEW_QUERY_KEY,
    enabled: import.meta.env.DEV && ready,
    retry: false,
    refetchInterval: eventError ? 500 : false,
    queryFn: async () => {
      const before = sequence.current;
      const open = await isStartupPreviewOpen();
      return sequence.current !== before ? latest.current : open;
    },
  });
  return { ...query, eventError, connecting: !ready };
}
