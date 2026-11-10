import { createFileRoute } from "@tanstack/react-router";
import { SearchXIcon } from "lucide-react";

import { EmptyState } from "@/components/ui/empty-state";
import { PageTransition } from "@/components/ui/page-transition";
import { parseModpackSource } from "@/features/discover/lib/discover-search";
import { ModpackPage } from "@/features/discover/modpack-page";

export const Route = createFileRoute("/_app/discover/$source/$projectId")({
  component: ModpackRoute,
});

function ModpackRoute() {
  const params = Route.useParams();
  const source = parseModpackSource(params.source);
  return (
    <PageTransition animationKey={`${params.source}/${params.projectId}`}>
      {source === null ? (
        <EmptyState
          icon={<SearchXIcon />}
          title="Modpack not found"
          description="This link points to an unknown modpack source."
        />
      ) : (
        <ModpackPage source={source} projectId={params.projectId} />
      )}
    </PageTransition>
  );
}
