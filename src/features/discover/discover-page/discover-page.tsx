import {
  CompassIcon,
  SearchIcon,
  SearchXIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { useCallback } from "react";

import { EmptyState } from "@/components/ui/empty-state";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { DiscoverFilters } from "@/features/discover/components/discover-filters";
import { DiscoverSort } from "@/features/discover/components/discover-sort";
import { ModpackList } from "@/features/discover/components/modpack-list";
import { useModpackSearch } from "@/features/discover/hooks/use-modpacks";
import { useSearchDraft } from "@/features/discover/hooks/use-search-draft";
import { friendlyError, messageOf } from "@/lib/errors";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";

import { discoverPageStyles } from "./discover-page.styles";

/** Store listing of Modrinth and CurseForge modpacks that loads as you scroll. */
export function DiscoverPage({
  query,
  filters,
  onSearch,
  onFiltersChange,
}: {
  query: string;
  filters: ModpackSearchFilters;
  onSearch: (query: string) => void;
  onFiltersChange: (filters: ModpackSearchFilters) => void;
}) {
  const search = useModpackSearch(query, filters);
  const { fetchNextPage } = search;
  // Calls while a page is loading are ignored instead of restarting it.
  const loadMore = useCallback(
    () => void fetchNextPage({ cancelRefetch: false }),
    [fetchNextPage],
  );
  const { draft, change } = useSearchDraft(query, onSearch);

  return (
    <div className={discoverPageStyles.root}>
      <header className={discoverPageStyles.header}>
        <h1 className={discoverPageStyles.title}>
          <CompassIcon aria-hidden className={discoverPageStyles.titleIcon} />
          Discover
        </h1>
        <div className={discoverPageStyles.controls}>
          <DiscoverSort
            value={filters.sort}
            onChange={(sort) => onFiltersChange({ ...filters, sort })}
          />
          <div className={discoverPageStyles.search}>
            <Input
              type="search"
              aria-label="Search modpacks"
              placeholder="Search modpacks"
              startIcon={<SearchIcon />}
              value={draft}
              onChange={(event) => change(event.target.value)}
            />
          </div>
        </div>
      </header>
      <div className={discoverPageStyles.body}>
        <DiscoverFilters filters={filters} onChange={onFiltersChange} />
        <section
          aria-label="Modpacks"
          aria-busy={search.isFetching}
          className={discoverPageStyles.results}
        >
          {search.isPending ? (
            <div className={discoverPageStyles.loading}>
              <Spinner />
            </div>
          ) : search.isError && !search.isFetchNextPageError ? (
            <EmptyState
              icon={<TriangleAlertIcon />}
              title="Could not load modpacks"
              description={friendlyError(messageOf(search.error))}
            />
          ) : search.data.length === 0 ? (
            <EmptyState
              icon={<SearchXIcon />}
              title="No modpacks found"
              description="Try a different search or fewer filters."
            />
          ) : (
            <ModpackList
              // A new search starts at the top of a fresh list.
              key={JSON.stringify([query, filters])}
              items={search.data}
              hasMore={search.hasNextPage}
              loadingMore={search.isFetchingNextPage}
              loadMoreFailed={search.isFetchNextPageError}
              onLoadMore={loadMore}
            />
          )}
        </section>
      </div>
    </div>
  );
}
