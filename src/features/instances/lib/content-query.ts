import { type QueryClient, queryOptions } from "@tanstack/react-query";
import { listInstanceContent } from "@/lib/commands";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

export function instanceContentQuery(
  client: QueryClient,
  folder: string,
  refresh = false,
) {
  const queryKey = ["instance-content", folder] as const;
  return queryOptions({
    queryKey,
    staleTime: Number.POSITIVE_INFINITY,
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
