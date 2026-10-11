import { createFileRoute } from "@tanstack/react-router";

import { InstancesPage } from "@/features/instances/instances-page";

export type HomeSearch = {
  prepare?: string;
};

export const Route = createFileRoute("/_app/")({
  validateSearch: (search: Record<string, unknown>): HomeSearch => ({
    prepare: typeof search.prepare === "string" ? search.prepare : undefined,
  }),
  component: InstancesPage,
});
