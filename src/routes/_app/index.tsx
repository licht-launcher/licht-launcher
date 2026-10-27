import { createFileRoute } from "@tanstack/react-router";

import { PageTransition } from "@/components/ui/page-transition";
import { useInstancesChrome } from "@/features/instances/components/instances-chrome-context";
import { InstancesPage } from "@/features/instances/instances-page";

export type HomeSearch = {
  prepare?: string;
  instance?: string;
};

export const Route = createFileRoute("/_app/")({
  validateSearch: (search: Record<string, unknown>): HomeSearch => ({
    prepare: typeof search.prepare === "string" ? search.prepare : undefined,
    instance: typeof search.instance === "string" ? search.instance : undefined,
  }),
  component: LibraryRoute,
});

function LibraryRoute() {
  const { selectedFolder } = useInstancesChrome();
  return (
    <PageTransition animationKey={selectedFolder}>
      <InstancesPage />
    </PageTransition>
  );
}
