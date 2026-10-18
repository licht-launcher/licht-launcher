import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
  DOWNLOAD_JOBS_QUERY_KEY,
  isDownloadFinished,
  replaceDownloadSnapshot,
} from "@/features/downloads/hooks/use-download-jobs";
import { VERSIONS_QUERY_KEY } from "@/features/instances/lib/versions-query";
import { listDownloadJobs } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";

export function useDownloadJobsSync() {
  const queryClient = useQueryClient();
  const [ready, setReady] = useState(false);
  const [live, setLive] = useState(true);
  const sequence = useRef(0);
  const latest = useRef(
    new Map<string, { sequence: number; snapshot: DownloadJobSnapshot }>(),
  );

  useEffect(() => {
    let stopped = false;
    const subscriptions: Array<() => void> = [];
    async function subscribe() {
      const updated = await listen<DownloadJobSnapshot>(
        "download-job-updated",
        ({ payload }) => {
          const previous = latest.current.get(payload.id)?.snapshot;
          latest.current.set(payload.id, {
            sequence: ++sequence.current,
            snapshot: payload,
          });
          queryClient.setQueryData<DownloadJobSnapshot[]>(
            DOWNLOAD_JOBS_QUERY_KEY,
            (current) => replaceDownloadSnapshot(current, payload),
          );
          if (
            isDownloadFinished(payload) ||
            payload.status === "failed" ||
            payload.versionId !== previous?.versionId ||
            (payload.phase === "downloading" && previous?.phase === "preparing")
          ) {
            void queryClient.invalidateQueries({ queryKey: ["instances"] });
            void queryClient.invalidateQueries({
              queryKey: VERSIONS_QUERY_KEY,
            });
          }
          if (
            payload.javaMajor !== undefined &&
            payload.status === "completed"
          ) {
            void queryClient.invalidateQueries({ queryKey: ["settings"] });
            void queryClient.invalidateQueries({ queryKey: ["runtime-java"] });
          }
        },
      );
      if (stopped) {
        updated();
        return;
      }
      subscriptions.push(updated);
      const invalidated = await listen("download-jobs-invalidated", () => {
        latest.current.clear();
        void queryClient.invalidateQueries({
          queryKey: DOWNLOAD_JOBS_QUERY_KEY,
        });
      });
      if (stopped) {
        invalidated();
        return;
      }
      subscriptions.push(invalidated);
      setReady(true);
    }
    void subscribe().catch((error: unknown) => {
      if (stopped) return;
      for (const unsubscribe of subscriptions) unsubscribe();
      subscriptions.length = 0;
      latest.current.clear();
      setLive(false);
      setReady(true);
      toast.error(friendlyError(messageOf(error)), { id: "download-events" });
    });
    return () => {
      stopped = true;
      for (const unsubscribe of subscriptions) unsubscribe();
    };
  }, [queryClient]);

  return useQuery({
    queryKey: DOWNLOAD_JOBS_QUERY_KEY,
    // The provider forwards the whole result, including to consumers mounted later.
    notifyOnChangeProps: "all",
    enabled: ready,
    retry: false,
    staleTime: 1000,
    refetchInterval: live ? false : 1000,
    queryFn: async () => {
      const before = sequence.current;
      const stored = await listDownloadJobs();
      let result = stored.filter((job) => !isDownloadFinished(job));
      for (const event of latest.current.values()) {
        if (event.sequence > before)
          result = replaceDownloadSnapshot(result, event.snapshot);
      }
      return result;
    },
  });
}
