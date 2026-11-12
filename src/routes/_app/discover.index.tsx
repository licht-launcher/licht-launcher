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
        filters={searchFilters(search)}
        onSearch={(query) =>
          void navigate({
            search: (current) => ({ ...current, q: query || undefined }),
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
