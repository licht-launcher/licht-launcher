import type { UseQueryResult } from "@tanstack/react-query";
import { createContext, useContext } from "react";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";

export const DOWNLOAD_JOBS_QUERY_KEY = ["download-jobs"] as const;
export const DownloadJobsContext = createContext<UseQueryResult<
  DownloadJobSnapshot[],
  Error
> | null>(null);

export function isDownloadFinished(job: DownloadJobSnapshot): boolean {
  return job.status === "completed" || job.status === "cancelled";
}

export function replaceDownloadSnapshot(
  current: DownloadJobSnapshot[] = [],
  snapshot: DownloadJobSnapshot,
): DownloadJobSnapshot[] {
  if (isDownloadFinished(snapshot))
    return current.filter((job) => job.id !== snapshot.id);
  if (!current.some((job) => job.id === snapshot.id))
    return [...current, snapshot];
  return current.map((job) => (job.id === snapshot.id ? snapshot : job));
}

export function useDownloadJobs() {
  const query = useContext(DownloadJobsContext);
  if (query === null) throw new Error("DownloadJobsProvider is missing");
  return query;
}
