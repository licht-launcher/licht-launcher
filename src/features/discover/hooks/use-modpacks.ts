import { useInfiniteQuery, useQuery } from "@tanstack/react-query";

import {
  getModpack,
  listModpackVersions,
  searchModpacks,
} from "@/api/commands";
import { mergeSearchPages } from "@/features/discover/lib/merge-pages";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";
import type { ModpackSource } from "@/lib/generated/ModpackSource";

/** Loaded results stay fresh long enough to come back from a pack's page. */
const SEARCH_STALE_TIME_MS = 5 * 60_000;

/**
 * The merged store search as one growing list. Pages load on demand through
 * `fetchNextPage`; `data` is the de-duplicated list of every loaded page.
 */
export function useModpackSearch(query: string, filters: ModpackSearchFilters) {
  return useInfiniteQuery({
    queryKey: ["modpacks", "search", query, filters],
    queryFn: ({ pageParam }) => searchModpacks(query, pageParam, filters),
    initialPageParam: 1,
    // Count from the page we asked for, so a reply cannot loop the list.
    getNextPageParam: (last, _pages, lastPageParam) =>
      lastPageParam < last.pageCount ? lastPageParam + 1 : undefined,
    select: (data) => mergeSearchPages(data.pages),
    staleTime: SEARCH_STALE_TIME_MS,
    // A refetch reloads every loaded page in turn, two APIs each; a new
    // search or a remount after `staleTime` is enough to refresh results.
    refetchOnWindowFocus: false,
  });
}

export function useModpack(source: ModpackSource, projectId: string) {
  return useQuery({
    queryKey: ["modpacks", "details", source, projectId],
    queryFn: () => getModpack(source, projectId),
  });
}

export function useModpackVersions(source: ModpackSource, projectId: string) {
  return useQuery({
    queryKey: ["modpacks", "versions", source, projectId],
    queryFn: () => listModpackVersions(source, projectId),
  });
}
