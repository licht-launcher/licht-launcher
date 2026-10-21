import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Update } from "@tauri-apps/plugin-updater";
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
import { invoke } from "@/features/settings/fixtures/invoke";
import {
  mockSettingsCommands,
  snapshot,
} from "@/features/settings/fixtures/render-settings";
import { promptTitleBarRestart } from "@/features/settings/lib/title-bar-restart-toast";
import { fromSnapshot } from "@/features/settings/model/draft";
import { UpdaterSection } from "@/features/settings/sections/updater-section";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { startupProgress } from "@/features/startup/fixtures/progress";
import {
  LATEST_VERSION_KEY,
  UpdateCheck,
} from "@/features/updater/components/update-check";
import { APP_VERSION } from "@/lib/app-release";
import type { StartupProgress } from "@/lib/generated/StartupProgress";

const { check, startupListeners } = vi.hoisted(() => ({
  check: vi.fn<() => Promise<Update | null>>(),
  startupListeners: new Map<
    string,
    (event: { payload: StartupProgress }) => void
  >(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    async (
      name: string,
      handler: (event: { payload: StartupProgress }) => void,
    ) => {
      startupListeners.set(name, handler);
      return () => {
        startupListeners.delete(name);
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

vi.mock("@tauri-apps/plugin-updater", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@tauri-apps/plugin-updater")>()),
  check,
}));

const settings = {
  ...snapshot,
  settings: { ...snapshot.settings, checkForUpdates: false },
};
const startupClient = new QueryClient({
  defaultOptions: { queries: { retry: false } },
});

function updaterView(showSection = true) {
  return (
    <QueryClientProvider client={startupClient}>
      <UpdateCheck />
      {showSection ? (
        <UpdaterSection
          draft={fromSnapshot(settings, null)}
          onChange={() => {}}
        />
      ) : null}
      <Toaster />
    </QueryClientProvider>
  );
}

async function renderUpdater() {
  const view = render(updaterView());
  await waitFor(() => {
    expect(
      screen.getByRole("button", { name: "Check for updates" }),
    ).toBeEnabled();
  });
  return view;
}

describe("UpdaterSection", () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
    check.mockReset();
    check.mockResolvedValue(null);
    startupClient.clear();
    startupListeners.clear();
    mockSettingsCommands((command) =>
      command === "get_settings"
        ? Promise.resolve(settings)
        : command === "get_startup_progress"
          ? Promise.resolve(startupProgress())
          : undefined,
    );
  });

  afterEach(() => {
    toast.dismiss();
    vi.restoreAllMocks();
  });

  it("shows an up-to-date result and a toast after a manual check", async () => {
    localStorage.setItem(LATEST_VERSION_KEY, "0.1.1");
    await renderUpdater();
    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));

    expect(await screen.findByText("Up to date")).toBeVisible();
    expect(screen.getByText("No newer version available")).toBeVisible();
    expect(
      await screen.findByText("Licht Launcher is up to date."),
    ).toBeVisible();
    expect(
      screen.getByText("No newer update is available for this installation."),
    ).toBeVisible();
    expect(check).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("0.1.1")).not.toBeInTheDocument();
  });

  it("stays unchecked when automatic checks are disabled", async () => {
    await renderUpdater();
    const section = within(
      screen.getByRole("region", { name: "Launcher updates" }),
    );
    expect(section.getByRole("status")).toHaveTextContent("Not checked yet");
    expect(check).not.toHaveBeenCalled();
  });

  it("shows progress and disables the button until the check finishes", async () => {
    let completeCheck: (value: Update | null) => void = () => {};
    check.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          completeCheck = resolve;
        }),
    );
    await renderUpdater();
    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));

    expect(screen.getByRole("button", { name: "Checking…" })).toBeDisabled();
    const section = within(
      screen.getByRole("region", { name: "Launcher updates" }),
    );
    expect(section.getByRole("status")).toHaveTextContent(
      "Checking for updates…",
    );
    const notifications = within(
      screen.getByRole("region", { name: /Notifications/ }),
    );
    expect(
      await notifications.findByText("Checking for updates…"),
    ).toBeVisible();
    expect(
      notifications.getByText(
        "Checking for a newer version of Licht Launcher.",
      ),
    ).toBeVisible();

    completeCheck(null);
    expect(await section.findByText("Up to date")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Check for updates" }),
    ).toBeEnabled();
  });

  it("shows the available version and the update toast", async () => {
    check.mockResolvedValueOnce(
      new Update({
        rid: 1,
        currentVersion: APP_VERSION,
        version: "0.2.2",
        rawJson: {},
      }),
    );
    await renderUpdater();
    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));

    const section = within(
      screen.getByRole("region", { name: "Launcher updates" }),
    );
    expect(await section.findByText("0.2.2")).toBeVisible();
    expect(section.getByRole("status")).toHaveTextContent("Update available");
    const buttons = section.getAllByRole("button");
    expect(buttons[0]).toHaveAccessibleName("Update");
    expect(buttons[1]).toHaveAccessibleName("Check for updates");
    expect(buttons[0]).toBeEnabled();
    expect(
      await screen.findByRole("button", { name: "Update now" }),
    ).toBeEnabled();
  });

  it("shows failure in the section and a toast, then allows a retry", async () => {
    check.mockRejectedValueOnce(new Error("Network unavailable"));
    await renderUpdater();
    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));

    const section = within(
      screen.getByRole("region", { name: "Launcher updates" }),
    );
    expect(
      await section.findByText("Update check failed. Try again."),
    ).toBeVisible();
    expect(section.getByText("Unavailable")).toBeVisible();
    expect(
      await screen.findByText(
        "Could not check for updates: Network unavailable",
      ),
    ).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(await section.findByText("Up to date")).toBeVisible();
    expect(check).toHaveBeenCalledTimes(2);
  });

  it("keeps the completed result when the settings section is reopened", async () => {
    const view = await renderUpdater();
    fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(await screen.findByText("Up to date")).toBeVisible();

    view.rerender(updaterView(false));
    view.rerender(updaterView());

    const section = within(
      screen.getByRole("region", { name: "Launcher updates" }),
    );
    expect(section.getByRole("status")).toHaveTextContent("Up to date");
    expect(section.getByText("No newer version available")).toBeVisible();
    expect(check).toHaveBeenCalledTimes(1);
  });

  it("shows the result of an automatic check without a success toast", async () => {
    mockSettingsCommands((command) =>
      command === "get_startup_progress"
        ? Promise.resolve(startupProgress({ updateStatus: "upToDate" }))
        : undefined,
    );
    await renderUpdater();

    expect(await screen.findByText("Up to date")).toBeVisible();
    expect(
      screen.queryByText("Licht Launcher is up to date."),
    ).not.toBeInTheDocument();
    expect(check).not.toHaveBeenCalled();
  });

  it("shows an automatic check already in progress when the section mounts", async () => {
    mockSettingsCommands((command) =>
      command === "get_startup_progress"
        ? Promise.resolve(startupProgress({ phase: "checkingUpdates" }))
        : undefined,
    );
    render(updaterView());

    expect(
      await screen.findByRole("button", { name: "Checking…" }),
    ).toBeDisabled();
    expect(check).not.toHaveBeenCalled();
    act(() =>
      startupListeners.get("startup-progress")?.({
        payload: startupProgress({ updateStatus: "upToDate" }),
      }),
    );
    expect(await screen.findByText("Up to date")).toBeVisible();
  });

  it("shows the update action when an automatic check finds a newer version", async () => {
    mockSettingsCommands((command) =>
      command === "get_startup_progress"
        ? Promise.resolve(
            startupProgress({ updateStatus: "available", version: "0.2.2" }),
          )
        : undefined,
    );

    render(updaterView());

    expect(await screen.findByText("0.2.2")).toBeVisible();
    expect(
      within(
        screen.getByRole("region", { name: "Launcher updates" }),
      ).getByRole("button", { name: "Update" }),
    ).toBeEnabled();
    expect(
      await screen.findByRole("button", { name: "Update now" }),
    ).toBeEnabled();
  });

  it("restarts through the native startup flow instead of installing inside the launcher", async () => {
    const download = vi.spyOn(Update.prototype, "downloadAndInstall");
    mockSettingsCommands((command) =>
      command === "get_startup_progress"
        ? Promise.resolve(
            startupProgress({ updateStatus: "available", version: "0.2.2" }),
          )
        : undefined,
    );
    await renderUpdater();
    const button = await screen.findByRole("button", { name: "Update now" });
    fireEvent.click(button);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("request_update_restart"),
    );
    expect(download).not.toHaveBeenCalled();
  });

  it("closes only the updater toast without dismissing settings or another alert", async () => {
    const onOpenChange = vi.fn();
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <QueryClientProvider client={queryClient}>
        <UpdateCheck />
        <SettingsDialog
          open
          onOpenChange={onOpenChange}
          initialSection="updater"
        />
        <Toaster />
      </QueryClientProvider>,
    );
    const button = await screen.findByRole("button", {
      name: "Check for updates",
    });
    await waitFor(() => {
      expect(button).toBeEnabled();
    });
    act(() => {
      promptTitleBarRestart();
    });
    expect(await screen.findByText("Restart required")).toBeVisible();
    fireEvent.click(button);
    expect(
      await screen.findByText("Licht Launcher is up to date."),
    ).toBeVisible();

    const notification = screen.getByText("Licht Launcher is up to date.");
    fireEvent.pointerDown(notification, { pointerType: "mouse", button: 0 });
    fireEvent.click(notification);
    expect(onOpenChange).not.toHaveBeenCalled();

    const close = screen.getByLabelText("Close update notification");
    fireEvent.pointerDown(close, { pointerType: "mouse", button: 0 });
    fireEvent.click(close);

    await waitFor(() => {
      expect(
        screen.queryByText("Licht Launcher is up to date."),
      ).not.toBeInTheDocument();
    });
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeVisible();
    expect(screen.getByText("Restart required")).toBeVisible();

    fireEvent.pointerDown(document.body, { pointerType: "mouse", button: 0 });
    fireEvent.click(document.body);
    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });
});
