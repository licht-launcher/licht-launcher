import { createFileRoute } from "@tanstack/react-router";

import { PageTransition } from "@/components/ui/page-transition";
import { DiscoverPage } from "@/features/discover/discover-page";
import {
  filterParams,
  parseDiscoverSearch,
  searchFilters,
} from "@/features/discover/lib/discover-search";

export const Route = createFileRoute("/_app/discover/")({
  validateSearch: parseDiscoverSearch,
  component: DiscoverRoute,
});

function DiscoverRoute() {
  const search = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <PageTransition>
      <DiscoverPage
        query={search.q ?? ""}
        page={search.page ?? 1}
        filters={searchFilters(search)}
        onSearch={(query) =>
          void navigate({
            search: (current) => ({
              ...current,
              q: query || undefined,
              page: undefined,
            }),
          })
        }
        onPageChange={(next) =>
          void navigate({
            search: (current) => ({
              ...current,
              page: next > 1 ? next : undefined,
            }),
          })
        }
        onFiltersChange={(filters) =>
          void navigate({
            search: (current) => ({
              q: current.q,
              ...filterParams(filters),
            }),
          })
        }
      />
    </PageTransition>
  );
}
