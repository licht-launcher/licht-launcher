import { createFileRoute } from "@tanstack/react-router";
import { DownloadsPage } from "@/features/downloads/downloads-page";

export const Route = createFileRoute("/_app/downloads")({
  component: DownloadsPage,
});
