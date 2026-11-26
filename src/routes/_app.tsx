import { createFileRoute, Outlet } from "@tanstack/react-router";
import { DownloadJobsProvider } from "@/features/downloads/components/download-jobs-provider";
import { InstancesChrome } from "@/features/instances/components/instances-chrome";
import { UpdateCheck } from "@/features/updater/components/update-check";
import { useSharedDataSync } from "@/hooks/use-shared-data-sync";

export const Route = createFileRoute("/_app")({
  component: AppLayout,
});

function AppLayout() {
  useSharedDataSync();
  return (
    <DownloadJobsProvider>
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <InstancesChrome>
          <Outlet />
        </InstancesChrome>
        <UpdateCheck />
      </div>
    </DownloadJobsProvider>
  );
}
