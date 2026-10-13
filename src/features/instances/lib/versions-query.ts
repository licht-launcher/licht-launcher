import { listVersions } from "@/lib/commands";

/** Shared key for the Mojang version manifest list. */
export const VERSIONS_QUERY_KEY = ["versions"] as const;

/**
 * Keep the manifest list until something invalidates it (install, create, …).
 * Avoids refetching (and WARN spam) on every route remount / Back navigation.
 */
export const VERSIONS_STALE_TIME = Number.POSITIVE_INFINITY;

export const versionsQueryOptions = {
  queryKey: VERSIONS_QUERY_KEY,
  queryFn: listVersions,
  retry: false as const,
  staleTime: VERSIONS_STALE_TIME,
};
