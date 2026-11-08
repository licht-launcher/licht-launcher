import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LauncherNavigation } from "@/components/title-bar/launcher-navigation";
import type { SectionId } from "@/features/settings/model/sections";

function renderNavigation(onOpenSettings: (section: SectionId) => void) {
  const routeTree = createRootRoute({
    component: () => (
      <LauncherNavigation
        libraryActive
        onOpenLibrary={() => {}}
        onOpenSettings={onOpenSettings}
      />
    ),
  });
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}

describe("LauncherNavigation settings handoff", () => {
  it.each(["pointer", "keyboard"])(
    "opens Settings after the menu has closed for %s activation",
    async (input) => {
      const menuWasClosed: boolean[] = [];
      const onOpenSettings = vi.fn(() =>
        menuWasClosed.push(
          screen.queryByRole("menu", { name: "Licht menu" }) === null,
        ),
      );
      renderNavigation(onOpenSettings);
      const trigger = await screen.findByRole("button", {
        name: "Licht Launcher",
      });
      trigger.focus();
      fireEvent.keyDown(trigger, { key: "Enter" });
      const menu = await screen.findByRole("menu", { name: "Licht menu" });
      const item = within(menu).getByRole("menuitem", { name: "Settings" });
      if (input === "keyboard") {
        item.focus();
        fireEvent.keyDown(item, { key: "Enter" });
      } else {
        fireEvent.pointerMove(item, { pointerType: "mouse" });
        fireEvent.pointerDown(item, { button: 0, pointerType: "mouse" });
        fireEvent.pointerUp(item, { button: 0, pointerType: "mouse" });
        fireEvent.click(item);
      }
      await waitFor(() =>
        expect(onOpenSettings).toHaveBeenCalledExactlyOnceWith("account"),
      );
      expect(menuWasClosed).toEqual([true]);
    },
  );
});
