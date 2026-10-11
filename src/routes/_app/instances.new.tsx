import { createFileRoute } from "@tanstack/react-router";

import { NewInstancePage } from "@/features/new-instance/new-instance-page";

export const Route = createFileRoute("/_app/instances/new")({
  component: NewInstancePage,
});
