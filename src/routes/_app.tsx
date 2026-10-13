import { createFileRoute, Outlet } from "@tanstack/react-router";

import { InstancesChrome } from "@/features/instances/components/instances-chrome";
import { UpdateCheck } from "@/features/updater/components/update-check";

export const Route = createFileRoute("/_app")({
  component: AppLayout,
});

function AppLayout() {
  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <InstancesChrome>
        <Outlet />
      </InstancesChrome>
      <UpdateCheck />
    </div>
  );
}
