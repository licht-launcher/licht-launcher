import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AppContextMenu } from "@/components/app-context-menu";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Toaster } from "@/components/ui/sonner";
import * as updater from "@/features/updater/components/update-check";
import { onOpenSettingsRequest } from "@/lib/shell-events";

const openRepository = vi.hoisted(() => vi.fn<() => Promise<void>>());

vi.mock("@/lib/commands", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/commands")>()),
  openRepository,
}));

async function renderApp() {
  const root = createRootRoute({
    component: () => (
      <AppContextMenu>
        <div>
          <button type="button">Menu target</button>
          <input aria-label="Text input" />
          <textarea aria-label="Text area" />
          <div contentEditable data-testid="rich-text" />
          <Dialog>
            <DialogTrigger asChild>
              <button type="button">Open dialog</button>
            </DialogTrigger>
            <DialogContent>
              <DialogTitle>Menu dialog</DialogTitle>
              <button type="button">Dialog target</button>
            </DialogContent>
          </Dialog>
          <Outlet />
        </div>
      </AppContextMenu>
    ),
  });
  const home = createRoute({
    getParentRoute: () => root,
    path: "/",
    component: () => <p>Home page</p>,
  });
  const next = createRoute({
    getParentRoute: () => root,
    path: "/instances/new",
    component: () => <p>Next page</p>,
  });
  const router = createRouter({
    routeTree: root.addChildren([home, next]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  await router.load();
  render(
    <QueryClientProvider
      client={
        new QueryClient({ defaultOptions: { queries: { retry: false } } })
      }
    >
      <RouterProvider router={router} />
      <Toaster />
    </QueryClientProvider>,
  );
  const target = await screen.findByRole("button", { name: "Menu target" });
  return { router, target };
}

async function openMenu(target: Element) {
  fireEvent.contextMenu(target, { clientX: 50, clientY: 60 });
  return screen.findByRole("menu", { name: "Application menu" });
}

describe("AppContextMenu", () => {
  beforeEach(() => {
    openRepository.mockReset();
    openRepository.mockResolvedValue();
    vi.spyOn(updater, "getUpdateCheckStatus").mockReturnValue("not-checked");
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("opens the application actions with unavailable history disabled", async () => {
    const { target } = await renderApp();
    const menu = await openMenu(target);
    expect(within(menu).queryByText("Licht Launcher")).not.toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Back" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
    expect(screen.getByRole("menuitem", { name: "Forward" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
    expect(screen.getByRole("menuitem", { name: "Settings" })).toBeVisible();
    expect(
      screen.getByRole("menuitem", { name: "Check for updates" }),
    ).toBeVisible();
    expect(screen.getByRole("menuitem", { name: "Open GitHub" })).toBeVisible();
  });

  it("navigates backward and forward through the existing router history", async () => {
    const { router, target } = await renderApp();
    await act(async () => {
      await router.navigate({ to: "/instances/new" });
    });
    expect(await screen.findByText("Next page")).toBeVisible();
    await openMenu(target);
    fireEvent.click(screen.getByRole("menuitem", { name: "Back" }));
    expect(await screen.findByText("Home page")).toBeVisible();
    await openMenu(target);
    fireEvent.click(screen.getByRole("menuitem", { name: "Forward" }));
    expect(await screen.findByText("Next page")).toBeVisible();
  });

  it("requests Settings using the existing shell event", async () => {
    const onSettings = vi.fn();
    const unsubscribe = onOpenSettingsRequest(onSettings);
    try {
      const { target } = await renderApp();
      await openMenu(target);
      fireEvent.click(screen.getByRole("menuitem", { name: "Settings" }));
      expect(onSettings).toHaveBeenCalledWith("account");
      await waitFor(() =>
        expect(screen.queryByRole("menu")).not.toBeInTheDocument(),
      );
    } finally {
      unsubscribe();
    }
  });

  it("requests a manual update check", async () => {
    const { target } = await renderApp();
    const dispatch = vi.spyOn(window, "dispatchEvent");
    await openMenu(target);
    fireEvent.click(
      screen.getByRole("menuitem", { name: "Check for updates" }),
    );
    expect(dispatch).toHaveBeenCalledWith(
      expect.objectContaining({ type: updater.MANUAL_UPDATE_CHECK_EVENT }),
    );
  });

  it("reflects an in-progress update check and prevents another request", async () => {
    const { target } = await renderApp();
    vi.mocked(updater.getUpdateCheckStatus).mockReturnValue("checking");
    const dispatch = vi.spyOn(window, "dispatchEvent");
    fireEvent(window, new Event(updater.UPDATE_CHECK_STATUS_EVENT));
    await openMenu(target);
    const item = screen.getByRole("menuitem", {
      name: "Checking for updates…",
    });
    expect(item).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(item);
    expect(dispatch).not.toHaveBeenCalledWith(
      expect.objectContaining({ type: updater.MANUAL_UPDATE_CHECK_EVENT }),
    );
  });

  it.each(["Text input", "Text area", "Rich text"])(
    "keeps native editing context actions in %s",
    async (label) => {
      await renderApp();
      const field =
        label === "Rich text"
          ? screen.getByTestId("rich-text")
          : screen.getByLabelText(label);
      expect(fireEvent.contextMenu(field)).toBe(true);
      expect(fireEvent.keyDown(field, { key: "ContextMenu" })).toBe(true);
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    },
  );

  it("opens the repository through the existing command", async () => {
    const { target } = await renderApp();
    await openMenu(target);
    fireEvent.click(screen.getByRole("menuitem", { name: "Open GitHub" }));
    await waitFor(() => expect(openRepository).toHaveBeenCalledTimes(1));
  });

  it("reports repository command errors using the existing toast", async () => {
    openRepository.mockRejectedValueOnce(new Error("I/O operation failed"));
    const { target } = await renderApp();
    await openMenu(target);
    fireEvent.click(screen.getByRole("menuitem", { name: "Open GitHub" }));
    expect(
      await screen.findByText(
        "The disk could not be written. Free some space and try again.",
      ),
    ).toBeVisible();
  });

  it("closes with Escape inside a dialog without dismissing the dialog", async () => {
    await renderApp();
    fireEvent.click(screen.getByRole("button", { name: "Open dialog" }));
    const target = await screen.findByRole("button", { name: "Dialog target" });
    const menu = await openMenu(target);
    fireEvent.keyDown(menu, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByRole("menu")).not.toBeInTheDocument(),
    );
    expect(screen.getByRole("dialog", { name: "Menu dialog" })).toBeVisible();
    await waitFor(() => expect(target).toHaveFocus());
  });
});
