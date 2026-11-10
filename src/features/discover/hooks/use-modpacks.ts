import { keepPreviousData, useQuery } from "@tanstack/react-query";

import {
  getModpack,
  listModpackVersions,
  searchModpacks,
} from "@/api/commands";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";
import type { ModpackSource } from "@/lib/generated/ModpackSource";

/** One page of the merged store search; keeps the previous page while paging. */
export function useModpackSearch(
  query: string,
  page: number,
  filters: ModpackSearchFilters,
) {
  return useQuery({
    queryKey: ["modpacks", "search", query, page, filters],
    queryFn: () => searchModpacks(query, page, filters),
    placeholderData: keepPreviousData,
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
