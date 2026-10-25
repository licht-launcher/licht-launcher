import type { ReactNode } from "react";
import { DownloadJobsContext } from "@/features/downloads/hooks/use-download-jobs";
import { useDownloadJobsSync } from "@/features/downloads/hooks/use-download-jobs-sync";
import { useDownloadTelemetrySync } from "@/features/downloads/hooks/use-download-telemetry-sync";

export function DownloadJobsProvider({ children }: { children: ReactNode }) {
  const query = useDownloadJobsSync();
  useDownloadTelemetrySync(
    query.data?.some(
      (job) => job.status === "running" && job.phase === "downloading",
    ) ?? false,
  );
  return (
    <DownloadJobsContext.Provider value={{ ...query }}>
      {children}
    </DownloadJobsContext.Provider>
  );
}
