import { createRootRoute, Outlet } from "@tanstack/react-router";

import { TitleBar } from "@/components/title-bar/title-bar";
import { useTitleBarDecorations } from "@/hooks/use-title-bar-decorations";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  const { disableCustomTitleBar } = useTitleBarDecorations();

  return (
    <div className="flex h-svh flex-col overflow-hidden">
      {disableCustomTitleBar ? null : <TitleBar />}
      <div className="min-h-0 flex-1 overflow-hidden">
        <Outlet />
      </div>
    </div>
  );
}
