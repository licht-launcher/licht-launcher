import { createFileRoute } from "@tanstack/react-router";
import { PageTransition } from "@/components/ui/page-transition";
import { DownloadsPage } from "@/features/downloads/downloads-page";

export const Route = createFileRoute("/_app/downloads")({
  component: DownloadsRoute,
});

function DownloadsRoute() {
  return (
    <PageTransition>
      <DownloadsPage />
    </PageTransition>
  );
}
