import { createFileRoute, Outlet } from "@tanstack/react-router";

import { InstancesChrome } from "@/features/instances/components/instances-chrome";

export const Route = createFileRoute("/_app")({
  component: AppLayout,
});

function AppLayout() {
  return (
    <InstancesChrome>
      <Outlet />
    </InstancesChrome>
  );
}
