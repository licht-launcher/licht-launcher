import { createFileRoute } from "@tanstack/react-router";

import { InstancesPage } from "@/features/instances/instances-page";

export const Route = createFileRoute("/")({
  component: InstancesPage,
});
