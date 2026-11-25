import { type QueryClient, queryOptions } from "@tanstack/react-query";
import { listInstanceContent } from "@/api/commands";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

/** Coming back to the app rereads the content folders at most this often. */
export const FOCUS_RELOAD_INTERVAL_MS = 15_000;

export function instanceContentQuery(
  client: QueryClient,
  folder: string,
  refresh = false,
) {
  const queryKey = ["instance-content", folder] as const;
  return queryOptions({
    queryKey,
    staleTime: Number.POSITIVE_INFINITY,
    // Files added or removed outside the app show up when the user comes back to it.
    // Unchanged files reuse their cached fingerprints, and quick back-and-forth
    // switching between windows does not reread the folders each time.
    refetchOnWindowFocus: (query: { state: { dataUpdatedAt: number } }) =>
      Date.now() - query.state.dataUpdatedAt >= FOCUS_RELOAD_INTERVAL_MS
        ? "always"
        : false,
    queryFn: ({ signal }) =>
      listInstanceContent(
        folder,
        refresh,
        (entries, initial) => {
          if (signal.aborted) return;
          client.setQueryData<InstanceContentEntry[]>(queryKey, (current) => {
            const known = new Map(
              (current ?? []).map((entry) => [
                `${entry.kind}:${entry.name}`,
                entry,
              ]),
            );
            if (initial) {
              return entries.map((entry) => {
                const cached = known.get(`${entry.kind}:${entry.name}`);
                return cached && cached.size === entry.size
                  ? {
                      ...entry,
                      title: cached.title,
                      version: cached.version,
                      iconUrl: cached.iconUrl,
                    }
                  : entry;
              });
            }
            const updates = new Map(
              entries.map((entry) => [`${entry.kind}:${entry.name}`, entry]),
            );
            return (current ?? []).map(
              (entry) => updates.get(`${entry.kind}:${entry.name}`) ?? entry,
            );
          });
        },
        signal,
      ),
  });
}
