import type { ReactNode } from "react";
import { DownloadJobsContext } from "@/features/downloads/hooks/use-download-jobs";
import { useDownloadJobsSync } from "@/features/downloads/hooks/use-download-jobs-sync";

export function DownloadJobsProvider({ children }: { children: ReactNode }) {
  const query = useDownloadJobsSync();
  return (
    <DownloadJobsContext.Provider value={{ ...query }}>
      {children}
    </DownloadJobsContext.Provider>
  );
}
