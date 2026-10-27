import { createFileRoute } from "@tanstack/react-router";
import { PageTransition } from "@/components/ui/page-transition";
import {
  type NewInstanceSearch,
  parseNewInstanceSearch,
} from "@/features/new-instance/model/wizard-search";
import { NewInstancePage } from "@/features/new-instance/new-instance-page";

export type { NewInstanceSearch };

export const Route = createFileRoute("/_app/instances/new")({
  validateSearch: (search: Record<string, unknown>): NewInstanceSearch =>
    parseNewInstanceSearch(search),
  component: NewInstanceRoute,
});

function NewInstanceRoute() {
  return (
    <PageTransition>
      <NewInstancePage />
    </PageTransition>
  );
}
