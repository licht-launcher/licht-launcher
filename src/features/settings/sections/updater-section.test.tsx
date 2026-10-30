import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Toaster } from "@/components/ui/sonner";
import {
  invoke,
  mockSettingsCommands,
  snapshot,
} from "@/features/settings/fixtures/render-settings";
import { promptTitleBarRestart } from "@/features/settings/lib/title-bar-restart-toast";
import { fromSnapshot } from "@/features/settings/model/draft";
import { UpdaterSection } from "@/features/settings/sections/updater-section";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { startupProgress } from "@/features/startup/fixtures/progress";
import {
  getUpdateCheckStatus,
  getUpdateSimulation,
  setUpdateSimulation,
  UpdateCheck,
} from "@/features/updater/components/update-check";
import { appUpdateState } from "@/features/updater/fixtures/state";
import { APP_UPDATE_QUERY_KEY } from "@/features/updater/hooks/use-app-update-state";
import { APP_VERSION } from "@/lib/app-release";
import type { AppUpdateState } from "@/lib/generated/AppUpdateState";

const { listeners, check, download, install } = vi.hoisted(() => ({
  listeners: new Map<string, Set<(event: { payload: unknown }) => void>>(),
  check: vi.fn(),
  download: vi.fn(),
  install: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
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
vi.mock("@tauri-apps/api/core", async (importOriginal) => {
  const original =
    await importOriginal<typeof import("@tauri-apps/api/core")>();
  const { invoke } = await import("@/features/settings/fixtures/invoke");
  return { ...original, invoke };
});

let state = appUpdateState({ automaticUpdates: false });
function emit(patch: Partial<AppUpdateState>) {
  state = { ...state, ...patch };
  act(() => {
    for (const handler of listeners.get("app-update-state") ?? [])
      handler({ payload: state });
  });
}

async function view(onChange = vi.fn()) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const result = render(
    <QueryClientProvider client={client}>
      <UpdateCheck />
      <UpdaterSection
        draft={fromSnapshot(
          {
            ...snapshot,
            settings: { ...snapshot.settings, automaticUpdates: false },
          },
          null,
        )}
        onChange={onChange}
      />
      <Toaster />
    </QueryClientProvider>,
  );
  await waitFor(() =>
    expect(client.getQueryState(APP_UPDATE_QUERY_KEY)?.status).toBe("success"),
  );
  return {
    ...result,
    client,
    section: within(screen.getByRole("region", { name: "Launcher updates" })),
  };
}

describe("UpdaterSection", () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
    listeners.clear();
    state = appUpdateState({ automaticUpdates: false });
    check.mockReset().mockImplementation(async () => {
      emit({ phase: "checking", error: null });
      emit({ phase: "upToDate", version: null, error: null });
    });
    download.mockReset().mockImplementation(async () => {
      emit({ phase: "downloading", error: null });
    });
    install.mockReset().mockImplementation(async () => {
      emit({ phase: "installing", error: null });
    });
    mockSettingsCommands((command) => {
      if (command === "get_app_update_state") return Promise.resolve(state);
      if (command === "get_startup_progress")
        return Promise.resolve(startupProgress());
      if (command === "check_app_update") return check();
      if (command === "download_app_update") return download();
      if (command === "install_app_update") return install();
      return undefined;
    });
  });
  afterEach(() => toast.dismiss());

  it("shows one automatic-update preference without starting frontend update checks", async () => {
    const onChange = vi.fn();
    const { section } = await view(onChange);
    expect(section.getAllByRole("switch")).toHaveLength(1);
    const automatic = section.getByRole("switch", {
      name: "Automatic updates",
    });
    expect(automatic).not.toBeChecked();
    fireEvent.click(automatic);
    expect(onChange).toHaveBeenCalledWith({ automaticUpdates: true });
    expect(check).not.toHaveBeenCalled();
  });

  it("requests a native manual check and reports the completed result", async () => {
    check.mockImplementationOnce(async () => {
      emit({ phase: "checking" });
    });
    const { section } = await view();
    fireEvent.click(section.getByRole("button", { name: "Check for updates" }));
    expect(
      await section.findByRole("button", { name: "Checking…" }),
    ).toBeDisabled();
    expect(section.getByRole("status")).toHaveTextContent(
      "Checking for updates…",
    );
    emit({ phase: "upToDate" });
    expect(await section.findByText("Up to date")).toBeVisible();
    expect(
      await screen.findByText("Licht Launcher is up to date."),
    ).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("check_app_update");
  });

  it("requires download and verification before offering restart", async () => {
    state = appUpdateState({
      phase: "available",
      automaticUpdates: false,
      version: "0.4.0",
    });
    const { section } = await view();
    expect(
      section.queryByRole("button", { name: "Restart to update" }),
    ).not.toBeInTheDocument();
    fireEvent.click(section.getByRole("button", { name: "Download update" }));
    await waitFor(() => expect(download).toHaveBeenCalledOnce());
    expect(
      section.getByRole("button", { name: "Downloading…" }),
    ).toBeDisabled();
    expect(install).not.toHaveBeenCalled();
    emit({ phase: "ready", downloadedBytes: 100, totalBytes: 100 });
    const restart = await section.findByRole("button", {
      name: "Restart to update",
    });
    expect(restart).toBeEnabled();
    expect(install).not.toHaveBeenCalled();
    fireEvent.click(restart);
    await waitFor(() => expect(install).toHaveBeenCalledOnce());
    expect(invoke).toHaveBeenCalledWith("install_app_update");
    expect(invoke).not.toHaveBeenCalledWith("request_update_restart");
  });

  it("uses native byte totals for background progress without triggering installation", async () => {
    state = appUpdateState({
      phase: "downloading",
      version: "0.4.0",
      downloadedBytes: 50,
      totalBytes: 100,
    });
    const { section } = await view();
    expect(
      section.getByRole("progressbar", { name: "Update download progress" }),
    ).toHaveAttribute("aria-valuenow", "50");
    emit({ phase: "verifying", downloadedBytes: 100 });
    expect(await section.findByText("Verifying update…")).toBeVisible();
    emit({ phase: "ready" });
    expect(
      await section.findByRole("button", { name: "Restart to update" }),
    ).toBeEnabled();
    await waitFor(() => expect(getUpdateCheckStatus()).toBe("available"));
    expect(check).not.toHaveBeenCalled();
    expect(install).not.toHaveBeenCalled();
  });

  it("does not invent a download percentage without a known size", async () => {
    state = appUpdateState({
      phase: "downloading",
      version: "0.4.0",
      downloadedBytes: 50,
    });
    const { section } = await view();
    expect(section.getByRole("progressbar")).not.toHaveAttribute(
      "aria-valuenow",
    );
  });

  it("prepares a cached package by checking metadata instead of downloading again", async () => {
    state = appUpdateState({
      phase: "cached",
      version: "0.4.0",
      downloadedBytes: 100,
      totalBytes: 100,
    });
    check.mockImplementationOnce(async () => {
      emit({ phase: "ready" });
    });
    const { section } = await view();
    expect(
      section.queryByRole("button", { name: "Restart to update" }),
    ).not.toBeInTheDocument();
    expect(section.getByText("Cached version: 0.4.0")).toBeVisible();
    fireEvent.click(section.getByRole("button", { name: "Prepare update" }));
    expect(
      await section.findByRole("button", { name: "Restart to update" }),
    ).toBeEnabled();
    expect(download).not.toHaveBeenCalled();
  });

  it("keeps native readiness when the section is reopened", async () => {
    state = appUpdateState({ phase: "ready", version: "0.4.0" });
    const first = await view();
    expect(
      first.section.getByRole("button", { name: "Restart to update" }),
    ).toBeEnabled();
    first.unmount();
    const second = await view();
    expect(
      second.section.getByRole("button", { name: "Restart to update" }),
    ).toBeEnabled();
    expect(download).not.toHaveBeenCalled();
    expect(install).not.toHaveBeenCalled();
  });

  it("shows native failures and allows a manual retry", async () => {
    check.mockImplementationOnce(async () => {
      emit({ phase: "failed", error: "Network unavailable" });
    });
    const { section } = await view();
    fireEvent.click(section.getByRole("button", { name: "Check for updates" }));
    expect(await section.findByRole("alert")).toHaveTextContent(
      "Network unavailable",
    );
    fireEvent.click(section.getByRole("button", { name: "Check for updates" }));
    expect(await section.findByText("Up to date")).toBeVisible();
    expect(check).toHaveBeenCalledTimes(2);
  });

  it("does not trust malformed progress events", async () => {
    const { section } = await view();
    act(() => {
      for (const handler of listeners.get("app-update-state") ?? [])
        handler({ payload: { phase: "ready", version: "forged" } });
    });
    expect(
      await section.findByText(
        "Live update progress is unavailable. Polling update status.",
      ),
    ).toBeVisible();
    expect(
      section.queryByRole("button", { name: "Restart to update" }),
    ).not.toBeInTheDocument();
  });

  it("keeps simulated updates active after reopening and opens only an isolated preview", async () => {
    const first = await view();
    act(() => setUpdateSimulation(true));
    await first.section.findByRole("button", { name: "Restart to update" });
    first.unmount();
    const { section } = await view();
    expect(section.getByRole("status")).toHaveTextContent(`${APP_VERSION}-dev`);
    fireEvent.click(section.getByRole("button", { name: "Restart to update" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("open_startup_preview", {
        scenario: "updateAndMigration",
      }),
    );
    expect(install).not.toHaveBeenCalled();
    act(() => setUpdateSimulation(false));
    expect(getUpdateSimulation()).toBe(false);
    expect(
      await section.findByRole("button", { name: "Check for updates" }),
    ).toBeEnabled();
  });

  it("dismisses only the manual update toast without dismissing settings or the restart prompt", async () => {
    const onOpenChange = vi.fn();
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <QueryClientProvider client={client}>
        <UpdateCheck />
        <SettingsDialog
          open
          onOpenChange={onOpenChange}
          initialSection="updater"
        />
        <Toaster />
      </QueryClientProvider>,
    );
    const section = within(
      await screen.findByRole("region", { name: "Launcher updates" }),
    );
    await waitFor(() =>
      expect(
        section.getByRole("button", { name: "Check for updates" }),
      ).toBeEnabled(),
    );
    act(() => promptTitleBarRestart());
    expect(await screen.findByText("Restart required")).toBeVisible();
    fireEvent.click(section.getByRole("button", { name: "Check for updates" }));
    await screen.findByText("Licht Launcher is up to date.");
    fireEvent.click(
      screen.getByRole("button", { name: "Close update notification" }),
    );
    await waitFor(() =>
      expect(
        screen.queryByText("Licht Launcher is up to date."),
      ).not.toBeInTheDocument(),
    );
    expect(screen.getByText("Restart required")).toBeVisible();
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeVisible();
    expect(onOpenChange).not.toHaveBeenCalled();
  });
});
