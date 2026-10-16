import { createRootRoute, Outlet } from "@tanstack/react-router";

import { AppContextMenu } from "@/components/app-context-menu";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  return (
    <AppContextMenu>
      <div className="flex h-svh flex-col overflow-hidden">
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <Outlet />
        </div>
      </div>
    </AppContextMenu>
  );
}
