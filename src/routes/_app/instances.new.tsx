import { createFileRoute } from "@tanstack/react-router";
import { PageTransition } from "@/components/ui/page-transition";
import { NewInstancePage } from "@/features/new-instance/new-instance-page";

export const Route = createFileRoute("/_app/instances/new")({
  component: NewInstanceRoute,
});

function NewInstanceRoute() {
  return (
    <PageTransition>
      <NewInstancePage />
    </PageTransition>
  );
}
