import { createFileRoute } from "@tanstack/react-router";

import { PageTransition } from "@/components/ui/page-transition";
import { DiscoverPage } from "@/features/discover/discover-page";
import { parseDiscoverSearch } from "@/features/discover/lib/discover-search";

export const Route = createFileRoute("/_app/discover/")({
  validateSearch: parseDiscoverSearch,
  component: DiscoverRoute,
});

function DiscoverRoute() {
  const { q, page } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <PageTransition>
      <DiscoverPage
        query={q ?? ""}
        page={page ?? 1}
        onSearch={(query) =>
          void navigate({ search: { q: query || undefined } })
        }
        onPageChange={(next) =>
          void navigate({
            search: (current) => ({
              ...current,
              page: next > 1 ? next : undefined,
            }),
          })
        }
      />
    </PageTransition>
  );
}
