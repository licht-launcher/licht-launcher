import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Toaster } from "@/components/ui/sonner";
import {
  invoke,
  mockSettingsCommands,
} from "@/features/settings/fixtures/render-settings";
import { SettingsWindow } from "@/features/settings/settings-window";
import { appUpdateState } from "@/features/updater/fixtures/state";
import type { AppUpdateState } from "@/lib/generated/AppUpdateState";

const { listeners, showWindow } = vi.hoisted(() => ({
  listeners: new Map<string, Set<(event: { payload: unknown }) => void>>(),
  showWindow: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  emitTo: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn(
    async (name: string, handler: (event: { payload: unknown }) => void) => {
      const handlers = listeners.get(name) ?? new Set();
      handlers.add(handler);
      listeners.set(name, handlers);
      return () => {
        handlers.delete(handler);
      };
    },
  ),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    show: showWindow,
    minimize: vi.fn(),
    close: vi.fn(),
    onCloseRequested: vi.fn().mockResolvedValue(() => {}),
  }),
}));
vi.mock("@tauri-apps/api/core", async (importOriginal) => {
  const original =
    await importOriginal<typeof import("@tauri-apps/api/core")>();
  const { invoke } = await import("@/features/settings/fixtures/invoke");
  return { ...original, invoke };
});

let state = appUpdateState({ automaticUpdates: false });

function emit(name: string, payload: unknown) {
  act(() => {
    for (const handler of listeners.get(name) ?? []) handler({ payload });
  });
}

function emitUpdate(patch: Partial<AppUpdateState>) {
  state = { ...state, ...patch };
  emit("app-update-state", state);
}

function view() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <SettingsWindow />
      <Toaster />
    </QueryClientProvider>,
  );
}

describe("SettingsWindow", () => {
  beforeEach(() => {
    listeners.clear();
    showWindow.mockReset().mockResolvedValue(undefined);
    state = appUpdateState({ automaticUpdates: false });
    mockSettingsCommands((command) => {
      if (command === "get_app_update_state") return Promise.resolve(state);
      if (command === "check_app_update") return Promise.resolve();
      return undefined;
    });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    toast.dismiss();
  });

  it("opens at the section the launcher asked for and shows itself", async () => {
    vi.stubGlobal("__LICHT_SETTINGS_SECTION__", "downloads");
    view();
    expect(
      await screen.findByRole("heading", { name: "Downloads" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Downloads", current: "page" }),
    ).toBeVisible();
    await waitFor(() => expect(showWindow).toHaveBeenCalledOnce());
  });

  it("opens General for an unknown section", async () => {
    vi.stubGlobal("__LICHT_SETTINGS_SECTION__", "nope");
    view();
    expect(
      await screen.findByRole("button", { name: "General", current: "page" }),
    ).toBeVisible();
  });

  it("switches section when the launcher asks for another one", async () => {
    view();
    await screen.findByRole("button", { name: "General", current: "page" });
    await waitFor(() =>
      expect(listeners.has("settings-window-section")).toBe(true),
    );
    emit("settings-window-section", "updater");
    expect(
      await screen.findByRole("button", { name: "Updates", current: "page" }),
    ).toBeVisible();
  });

  it("refetches settings saved by another window", async () => {
    view();
    await screen.findByRole("button", { name: "Dark" });
    const fetches = () =>
      invoke.mock.calls.filter(([command]) => command === "get_settings")
        .length;
    const before = fetches();
    await waitFor(() => expect(listeners.has("settings-changed")).toBe(true));
    emit("settings-changed", null);
    await waitFor(() => expect(fetches()).toBeGreaterThan(before));
  });

  it("checks for updates from this window and reports the result here", async () => {
    vi.stubGlobal("__LICHT_SETTINGS_SECTION__", "updater");
    view();
    fireEvent.click(
      await screen.findByRole("button", { name: "Check for updates" }),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("check_app_update"),
    );
    expect(await screen.findByText("Checking for updates…")).toBeVisible();
    emitUpdate({ phase: "upToDate", version: null, error: null });
    expect(
      await screen.findByText("Licht Launcher is up to date."),
    ).toBeVisible();
  });

  it("closes the checking notice when the update is ready", async () => {
    vi.stubGlobal("__LICHT_SETTINGS_SECTION__", "updater");
    view();
    fireEvent.click(
      await screen.findByRole("button", { name: "Check for updates" }),
    );
    expect(await screen.findByText("Checking for updates…")).toBeVisible();
    emitUpdate({ phase: "ready", version: "9.9.9", error: null });
    await waitFor(() =>
      expect(
        screen.queryByText("Checking for updates…"),
      ).not.toBeInTheDocument(),
    );
    expect(screen.queryByText("Update ready")).not.toBeInTheDocument();
  });
});
