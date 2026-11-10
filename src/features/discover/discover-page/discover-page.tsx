import {
  CompassIcon,
  SearchIcon,
  SearchXIcon,
  TriangleAlertIcon,
} from "lucide-react";

import { EmptyState } from "@/components/ui/empty-state";
import { Input } from "@/components/ui/input";
import { Pagination } from "@/components/ui/pagination";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { DiscoverFilters } from "@/features/discover/components/discover-filters";
import { ModpackRow } from "@/features/discover/components/modpack-row";
import { useModpackSearch } from "@/features/discover/hooks/use-modpacks";
import { useSearchDraft } from "@/features/discover/hooks/use-search-draft";
import { friendlyError, messageOf } from "@/lib/errors";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";

import { discoverPageStyles } from "./discover-page.styles";

/** Store listing of Modrinth and CurseForge modpacks, one page at a time. */
export function DiscoverPage({
  query,
  page,
  filters,
  onSearch,
  onPageChange,
  onFiltersChange,
}: {
  query: string;
  page: number;
  filters: ModpackSearchFilters;
  onSearch: (query: string) => void;
  onPageChange: (page: number) => void;
  onFiltersChange: (filters: ModpackSearchFilters) => void;
}) {
  const search = useModpackSearch(query, page, filters);
  const { draft, change } = useSearchDraft(query, onSearch);

  return (
    <div className={discoverPageStyles.root}>
      <header className={discoverPageStyles.header}>
        <h1 className={discoverPageStyles.title}>
          <CompassIcon aria-hidden className={discoverPageStyles.titleIcon} />
          Discover
        </h1>
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
      </header>
      <div className={discoverPageStyles.body}>
        <DiscoverFilters filters={filters} onChange={onFiltersChange} />
        <div className={discoverPageStyles.main}>
          <section
            aria-label="Modpacks"
            aria-busy={search.isFetching}
            className={discoverPageStyles.results}
          >
            {search.isPending ? (
              <div className={discoverPageStyles.loading}>
                <Spinner />
              </div>
            ) : search.isError ? (
              <EmptyState
                icon={<TriangleAlertIcon />}
                title="Could not load modpacks"
                description={friendlyError(messageOf(search.error))}
              />
            ) : search.data.items.length === 0 ? (
              <EmptyState
                icon={<SearchXIcon />}
                title="No modpacks found"
                description="Try a different search."
              />
            ) : (
              <ScrollArea key={page} className={discoverPageStyles.scrollArea}>
                <ul className={discoverPageStyles.list}>
                  {search.data.items.map((modpack) => (
                    <li key={`${modpack.source}:${modpack.id}`}>
                      <ModpackRow modpack={modpack} />
                    </li>
                  ))}
                </ul>
              </ScrollArea>
            )}
          </section>
          <footer className={discoverPageStyles.footer}>
            <Pagination
              page={page}
              pageCount={search.data?.pageCount ?? 0}
              onPageChange={onPageChange}
              disabled={search.isFetching}
            />
          </footer>
        </div>
      </div>
    </div>
  );
}
