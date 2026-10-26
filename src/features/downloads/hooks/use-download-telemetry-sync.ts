import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef } from "react";
import { SPEED_HISTORY_MS, SPEED_SAMPLE_MS } from "@/components/ui/speed-chart";
import {
  DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
  DOWNLOAD_TELEMETRY_QUERY_KEY,
  DOWNLOAD_TELEMETRY_STATUS_QUERY_KEY,
  EMPTY_SPEED_HISTORY,
  type SpeedHistory,
} from "@/features/downloads/hooks/use-download-telemetry";
import { getDownloadTelemetry } from "@/lib/commands";
import type { DownloadTelemetry } from "@/lib/generated/DownloadTelemetry";

export function useDownloadTelemetrySync(active: boolean) {
  const client = useQueryClient();
  const activeRef = useRef(active);
  const latest = useRef<DownloadTelemetry | null>(null);
  const unavailable = useRef(false);
  const previous = useRef<{ at: number; telemetry: DownloadTelemetry } | null>(
    null,
  );
  const status = useQuery({
    queryKey: DOWNLOAD_TELEMETRY_STATUS_QUERY_KEY,
    queryFn: () => false,
    initialData: false,
    enabled: false,
    gcTime: Infinity,
  });
  useQuery({
    queryKey: DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
    queryFn: () => EMPTY_SPEED_HISTORY,
    initialData: EMPTY_SPEED_HISTORY,
    enabled: false,
    gcTime: Infinity,
  });
  const fallback = useQuery({
    queryKey: DOWNLOAD_TELEMETRY_QUERY_KEY,
    queryFn: getDownloadTelemetry,
    enabled: status.data,
    retry: false,
    refetchInterval: status.data ? SPEED_SAMPLE_MS : false,
  });

  const accept = useCallback((payload: DownloadTelemetry) => {
    // A command response or IPC event can arrive behind a newer counter snapshot.
    if (
      latest.current &&
      (payload.receivedBytes < latest.current.receivedBytes ||
        payload.writtenBytes < latest.current.writtenBytes)
    )
      return;
    latest.current = payload;
    if (activeRef.current && previous.current === null)
      previous.current = { at: performance.now(), telemetry: payload };
  }, []);

  useEffect(() => {
    activeRef.current = active;
    previous.current =
      active && latest.current
        ? { at: performance.now(), telemetry: latest.current }
        : null;
  }, [active]);

  useEffect(() => {
    if (
      fallback.data &&
      !fallback.isError &&
      (status.data || latest.current === null)
    )
      accept(fallback.data);
  }, [fallback.data, fallback.isError, status.data, accept]);

  useEffect(() => {
    unavailable.current = status.data && fallback.isError;
    if (unavailable.current) previous.current = null;
  }, [status.data, fallback.isError]);

  useEffect(() => {
    let stopped = false;
    let unlisten: (() => void) | undefined;
    void listen<DownloadTelemetry>("download-telemetry", ({ payload }) => {
      if (stopped) return;
      accept(payload);
      const cached = client.getQueryData<DownloadTelemetry>(
        DOWNLOAD_TELEMETRY_QUERY_KEY,
      );
      if (
        cached?.receivedBytes !== payload.receivedBytes ||
        cached.writtenBytes !== payload.writtenBytes ||
        cached.paused !== payload.paused
      )
        client.setQueryData(DOWNLOAD_TELEMETRY_QUERY_KEY, payload);
    })
      .then((dispose) => {
        if (stopped) dispose();
        else {
          unlisten = dispose;
          client.setQueryData(DOWNLOAD_TELEMETRY_STATUS_QUERY_KEY, false);
        }
      })
      .catch(() => {
        if (!stopped)
          client.setQueryData(DOWNLOAD_TELEMETRY_STATUS_QUERY_KEY, true);
      });

    const timer = window.setInterval(() => {
      if (!activeRef.current || !latest.current || unavailable.current) return;
      const now = performance.now();
      const at = Math.floor(now / SPEED_SAMPLE_MS) * SPEED_SAMPLE_MS;
      const last = previous.current;
      const history =
        client.getQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY) ??
        EMPTY_SPEED_HISTORY;
      if ((history.received.at(-1)?.at ?? -Infinity) >= at) return;
      previous.current = { at: now, telemetry: latest.current };
      if (!last || now <= last.at) return;
      // An initial partial slice still belongs to one full second on the graph.
      const seconds = Math.max(SPEED_SAMPLE_MS, now - last.at) / 1000;
      const received =
        Math.max(
          0,
          latest.current.receivedBytes - last.telemetry.receivedBytes,
        ) / seconds;
      const written =
        Math.max(0, latest.current.writtenBytes - last.telemetry.writtenBytes) /
        seconds;
      client.setQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY, {
        received: [
          ...history.received.filter(
            (point) => point.at > at - SPEED_HISTORY_MS,
          ),
          { at, bytesPerSecond: received },
        ],
        written: [
          ...history.written.filter(
            (point) => point.at > at - SPEED_HISTORY_MS,
          ),
          { at, bytesPerSecond: written },
        ],
      });
    }, SPEED_SAMPLE_MS);

    return () => {
      stopped = true;
      window.clearInterval(timer);
      unlisten?.();
    };
  }, [client, accept]);
}
