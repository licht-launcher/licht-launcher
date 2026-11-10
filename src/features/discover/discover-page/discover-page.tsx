import {
  CircleAlertIcon,
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
import {
  useModpackSearch,
  useModpackSources,
} from "@/features/discover/hooks/use-modpacks";
import { useSearchDraft } from "@/features/discover/hooks/use-search-draft";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import { friendlyError, messageOf } from "@/lib/errors";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";
import type { ModpackSource } from "@/lib/generated/ModpackSource";

import { discoverPageStyles } from "./discover-page.styles";

const MODRINTH_ONLY: readonly ModpackSource[] = ["modrinth"];

function unavailableNotice(sources: readonly ModpackSource[]): string {
  const names = sources.map((source) => SOURCE_LABELS[source]).join(" and ");
  return `${names} results couldn't load right now. Showing the rest.`;
}

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
  const offered = useModpackSources();
  // The sources call is local; if it ever fails, Modrinth alone still works.
  const sources = offered.data ?? (offered.isError ? MODRINTH_ONLY : null);
  // A link may ask for a source this build cannot search; it is ignored.
  const effectiveFilters =
    filters.source !== null && !sources?.includes(filters.source)
      ? { ...filters, source: null }
      : filters;
  const search = useModpackSearch(query, effectiveFilters, sources !== null);
  const unavailable = search.data?.unavailableSources ?? [];
  const listKey = JSON.stringify([query, effectiveFilters]);
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
        <DiscoverFilters
          filters={effectiveFilters}
          sources={sources ?? MODRINTH_ONLY}
          onChange={onFiltersChange}
        />
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
          ) : search.data.items.length === 0 ? (
            <EmptyState
              icon={<SearchXIcon />}
              title="No modpacks found"
              description="Try a different search or fewer filters."
            />
          ) : (
            <>
              {unavailable.length > 0 && (
                <p role="status" className={discoverPageStyles.notice}>
                  <CircleAlertIcon aria-hidden />
                  {unavailableNotice(unavailable)}
                </p>
              )}
              <ModpackList
                // A new search starts at the top of a fresh list.
                key={listKey}
                positionKey={listKey}
                items={search.data.items}
                hasMore={search.hasNextPage}
                loadingMore={search.isFetchingNextPage}
                loadMoreFailed={search.isFetchNextPageError}
                onLoadMore={loadMore}
              />
            </>
          )}
        </section>
      </div>
    </div>
  );
}
