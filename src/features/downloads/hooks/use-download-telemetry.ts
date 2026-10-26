import { useQuery } from "@tanstack/react-query";
import type { SpeedSample } from "@/components/ui/speed-chart";
import { getDownloadTelemetry } from "@/lib/commands";

export const DOWNLOAD_TELEMETRY_QUERY_KEY = ["download-telemetry"] as const;
export const DOWNLOAD_SPEED_HISTORY_QUERY_KEY = [
  "download-speed-history",
] as const;
export const DOWNLOAD_TELEMETRY_STATUS_QUERY_KEY = [
  "download-telemetry-status",
] as const;
export type SpeedHistory = { received: SpeedSample[]; written: SpeedSample[] };
export const EMPTY_SPEED_HISTORY: SpeedHistory = { received: [], written: [] };

export function useDownloadTelemetry() {
  const status = useQuery({
    queryKey: DOWNLOAD_TELEMETRY_STATUS_QUERY_KEY,
    queryFn: () => false,
    initialData: false,
    enabled: false,
    gcTime: Infinity,
  });
  const history = useQuery({
    queryKey: DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
    queryFn: () => EMPTY_SPEED_HISTORY,
    initialData: EMPTY_SPEED_HISTORY,
    enabled: false,
    gcTime: Infinity,
  });
  const query = useQuery({
    queryKey: DOWNLOAD_TELEMETRY_QUERY_KEY,
    queryFn: getDownloadTelemetry,
    retry: false,
  });

  return {
    data: query.data,
    isError: query.isError,
    samples: history.data,
    subscriptionError: status.data,
  };
}
