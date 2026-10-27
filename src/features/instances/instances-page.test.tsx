import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Toaster } from "@/components/ui/sonner";
import {
  completedDownload,
  downloadJob,
} from "@/features/downloads/fixtures/job";
import { storageSummary } from "@/features/downloads/fixtures/storage";
import {
  DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
  type SpeedHistory,
} from "@/features/downloads/hooks/use-download-telemetry";
import {
  playerHeadPathFixture,
  snapshot,
} from "@/features/settings/fixtures/snapshot";
import { startupProgress } from "@/features/startup/fixtures/progress";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { GameLogEvent } from "@/lib/generated/GameLogEvent";
import type { InstallProgress } from "@/lib/generated/InstallProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import { requestOpenSettings } from "@/lib/shell-events";
import { routeTree } from "@/routeTree.gen";

const { invoke, listeners, closeWindow } = vi.hoisted(() => ({
  invoke: vi.fn(),
  closeWindow: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    async (name: string, handler: (event: { payload: unknown }) => void) => {
      listeners.set(name, handler);
      return () => {
        listeners.delete(name);
      };
    },
  ),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    close: closeWindow,
    setDecorations: vi.fn(),
  }),
}));

async function renderAt(path: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  await router.load();
  const view = await act(async () =>
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        <Toaster />
      </QueryClientProvider>,
    ),
  );
  return { ...view, router, queryClient };
}

function defaultCommandResponse(command: string): Promise<unknown> {
  switch (command) {
    case "get_player_head":
      return Promise.resolve(playerHeadPathFixture);
    case "get_startup_progress":
      return Promise.resolve(startupProgress());
    case "download_telemetry":
      return Promise.resolve({
        receivedBytes: 0,
        writtenBytes: 0,
        paused: false,
      });
    case "custom_title_bar_active":
      return Promise.resolve(true);
    case "storage_summary":
      return Promise.resolve(storageSummary);
    case "list_instance_content":
    case "list_download_jobs":
    case "list_microsoft_accounts":
      return Promise.resolve([]);
    case "active_microsoft_account":
      return Promise.resolve(null);
    case "prepare_instance_download":
      return Promise.resolve(completedDownload());
    case "get_last_played_folder":
      return Promise.resolve(null);
    default:
      return Promise.resolve(undefined);
  }
}

const classicInstance = {
  folder: "classic",
  name: "Classic",
  versionId: "1.5.2",
  minMemoryMb: 512,
  maxMemoryMb: 2048,
  jvmArguments: [],
  fullscreen: false,
  width: null,
  height: null,
  overrideWindow: false,
  overrideMemory: false,
  overrideJava: false,
  overrideJvmArguments: false,
  javaPath: null,
  lastOpenedAt: null as string | null,
  iconPath: null as string | null,
};

const contextInstance = {
  ...classicInstance,
  folder: "next",
  name: "Next",
  versionId: "1.20.1",
};

function mockContextMenuCommands(
  implement?: (command: string, args?: unknown) => unknown,
) {
  let entries = [classicInstance, contextInstance];
  invoke.mockImplementation((command: string, args?: unknown) => {
    const custom = implement?.(command, args);
    if (custom !== undefined) return custom;
    if (command === "list_instances") return Promise.resolve(entries);
    if (command === "get_settings") {
      return Promise.resolve({
        ...snapshot,
        settings: { ...snapshot.settings, checkForUpdates: false },
      });
    }
    if (command === "list_running_games") return Promise.resolve([]);
    if (command === "get_last_played_folder") return Promise.resolve("classic");
    if (command === "is_version_ready_cmd") return Promise.resolve(true);
    if (command === "list_versions") {
      return Promise.resolve([
        { id: "1.5.2", versionType: "release", installed: true },
        { id: "1.20.1", versionType: "release", installed: true },
      ]);
    }
    const input = args !== null && typeof args === "object" ? args : {};
    const folder = "folder" in input ? input.folder : undefined;
    const name = "name" in input ? input.name : undefined;
    const entry = entries.find((item) => item.folder === folder);
    if (command === "touch_last_opened" && entry) {
      return Promise.resolve({
        ...entry,
        lastOpenedAt: "2026-10-15T12:00:00Z",
      });
    }
    if (command === "rename_instance" && entry && typeof name === "string") {
      const renamed = { ...entry, name };
      entries = entries.map((item) =>
        item.folder === folder ? renamed : item,
      );
      return Promise.resolve(renamed);
    }
    if (command === "duplicate_instance" && entry && typeof name === "string") {
      const duplicate = { ...entry, folder: "next-copy", name };
      entries = [...entries, duplicate];
      return Promise.resolve(duplicate);
    }
    if (command === "delete_instance") {
      entries = entries.filter((item) => item.folder !== folder);
      return Promise.resolve();
    }
    return defaultCommandResponse(command);
  });
}

async function openInstanceContextMenu(name = "Next") {
  const sidebar = await screen.findByRole("navigation", { name: "Instances" });
  const target = await within(sidebar).findByRole("button", {
    name: new RegExp(`^${name}(?: \\(|$)`),
  });
  fireEvent.contextMenu(target, { clientX: 50, clientY: 60 });
  return screen.findByRole("menu", { name: `Actions for ${name}` });
}

describe("InstancesPage", () => {
  beforeEach(() => {
    listeners.clear();
    invoke.mockReset();
    closeWindow.mockReset().mockResolvedValue(undefined);
    vi.spyOn(console, "error");
  });

  afterEach(() => {
    cleanup();
    const errors = vi
      .mocked(console.error)
      .mock.calls.filter((args) =>
        args.some((arg) =>
          /Query data cannot be undefined|Maximum update depth exceeded/.test(
            String(arg),
          ),
        ),
      );
    vi.mocked(console.error).mockRestore();
    expect(errors).toEqual([]);
  });

  it("opens Updater settings from the version and codename in the footer", async () => {
    mockContextMenuCommands((command) =>
      command === "runtime_java"
        ? Promise.resolve({
            java25: null,
            java21: null,
            java17: null,
            java8: null,
          })
        : undefined,
    );
    await renderAt("/");
    const footer = within(
      screen.getByRole("contentinfo", { name: "Download status" }),
    );
    const version = footer.getByRole("button", {
      name: "Early Release 0.3.0 · Rose",
    });
    expect(version).toBeVisible();
    fireEvent.click(version);
    const settings = await screen.findByRole("dialog", { name: "Settings" });
    expect(
      within(settings).getByRole("button", {
        name: "Updater",
        current: "page",
      }),
    ).toBeVisible();
  });

  it("opens full-width Downloads from the footer and restores the library sidebar", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/");
    await screen.findByRole("heading", { name: "Classic" });
    const navigation = within(
      screen.getByRole("navigation", { name: "Main navigation" }),
    );
    expect(
      navigation.queryByRole("button", { name: "Downloads" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Instances" })).toBeVisible();
    const footer = within(
      screen.getByRole("contentinfo", { name: "Download status" }),
    );
    const downloads = await footer.findByRole("button", {
      name: "Downloads: Idle",
    });
    await act(async () => {
      fireEvent.click(downloads);
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "Downloads" }),
    ).toBeVisible();
    expect(router.state.location.pathname).toBe("/downloads");
    expect(
      screen.queryByRole("navigation", { name: "Instances" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("separator", { name: "Resize sidebar" }),
    ).not.toBeInTheDocument();
    await act(async () => {
      fireEvent.click(navigation.getByRole("button", { name: "Library" }));
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeVisible();
    expect(router.state.location.pathname).toBe("/");
    expect(screen.getByRole("navigation", { name: "Instances" })).toBeVisible();
  });

  it("keeps the Downloads route open when the library restores its selection", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/downloads");
    expect(
      await screen.findByRole("heading", { name: "Downloads" }),
    ).toBeVisible();
    expect(await screen.findByText("No downloads in progress")).toBeVisible();
    expect(router.state.location.pathname).toBe("/downloads");
    expect(
      screen.queryByRole("navigation", { name: "Instances" }),
    ).not.toBeInTheDocument();
  });

  it("continues collecting speed history in Library and restores it when returning to Downloads", async () => {
    mockContextMenuCommands((command) =>
      command === "list_download_jobs"
        ? Promise.resolve([downloadJob({ instanceFolder: "classic" })])
        : undefined,
    );
    const { router, queryClient } = await renderAt("/downloads");
    await screen.findByRole("region", { name: "Download for Classic" });
    await waitFor(() => expect(listeners.has("download-telemetry")).toBe(true));
    act(() =>
      listeners.get("download-telemetry")?.({
        payload: {
          receivedBytes: 1_000_000,
          writtenBytes: 500_000,
          paused: false,
        },
      }),
    );
    await waitFor(
      () =>
        expect(
          queryClient.getQueryData<SpeedHistory>(
            DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
          )?.received.length,
        ).toBeGreaterThan(0),
      { timeout: 3000 },
    );
    const first = queryClient.getQueryData<SpeedHistory>(
      DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
    )?.received[0];
    const before =
      queryClient.getQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY)
        ?.received.length ?? 0;
    await act(async () => {
      fireEvent.click(
        within(
          screen.getByRole("navigation", { name: "Main navigation" }),
        ).getByRole("button", { name: "Library" }),
      );
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("region", { name: "Transfer speed" }),
    ).not.toBeInTheDocument();
    act(() =>
      listeners.get("download-telemetry")?.({
        payload: {
          receivedBytes: 2_000_000,
          writtenBytes: 1_000_000,
          paused: false,
        },
      }),
    );
    await waitFor(
      () =>
        expect(
          queryClient.getQueryData<SpeedHistory>(
            DOWNLOAD_SPEED_HISTORY_QUERY_KEY,
          )?.received.length,
        ).toBeGreaterThan(before),
      { timeout: 3000 },
    );
    await act(async () => {
      fireEvent.click(
        within(
          screen.getByRole("contentinfo", { name: "Download status" }),
        ).getByRole("button", { name: /^Downloads:/ }),
      );
      await router.load();
    });
    expect(
      await screen.findByRole("region", { name: "Transfer speed" }),
    ).toBeVisible();
    expect(
      queryClient.getQueryData<SpeedHistory>(DOWNLOAD_SPEED_HISTORY_QUERY_KEY)
        ?.received[0],
    ).toEqual(first);
    expect(
      screen.getByLabelText("Download bars").querySelectorAll("rect").length,
    ).toBeGreaterThanOrEqual(2);
  });

  it("opens the Downloads settings directly from the downloads screen", async () => {
    mockContextMenuCommands((command) =>
      command === "runtime_java"
        ? Promise.resolve({
            java25: null,
            java21: null,
            java17: null,
            java8: null,
          })
        : undefined,
    );
    const { router } = await renderAt("/downloads");
    fireEvent.click(
      await screen.findByRole("button", { name: "Download settings" }),
    );
    const settings = await screen.findByRole("dialog", { name: "Settings" });
    expect(
      within(settings).getByRole("button", {
        name: "Downloads",
        current: "page",
      }),
    ).toBeVisible();
    expect(router.state.location.pathname).toBe("/downloads");
  });

  it("opens the developer loading test in Downloads and closes Settings", async () => {
    mockContextMenuCommands((command) =>
      command === "runtime_java"
        ? Promise.resolve({
            java25: null,
            java21: null,
            java17: null,
            java8: null,
          })
        : undefined,
    );
    const { router } = await renderAt("/");
    act(() => requestOpenSettings("developers"));
    const settings = await screen.findByRole("dialog", { name: "Settings" });
    const preview = await within(settings).findByRole("button", {
      name: "Test Launcher Data loading",
    });
    await act(async () => {
      fireEvent.click(preview);
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "Downloads" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("dialog", { name: "Settings" }),
    ).not.toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/downloads");
    const data = within(screen.getByRole("region", { name: "Launcher Data" }));
    expect(data.getAllByRole("status")).toHaveLength(6);
    expect(data.getByText("Instances")).toBeVisible();
    expect(invoke).not.toHaveBeenCalledWith("storage_summary");
  });

  it("opens the instance creator from the bottom bar shortcut", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/");
    const footer = screen.getByRole("contentinfo", { name: "Download status" });
    const create = within(footer).getByRole("link", {
      name: "Create instance",
    });
    expect(create).toBeVisible();
    expect(
      within(screen.getByRole("navigation", { name: "Instances" })).queryByRole(
        "button",
        { name: "New instance" },
      ),
    ).not.toBeInTheDocument();
    await act(async () => {
      fireEvent.click(create);
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeVisible();
    expect(router.state.location.pathname).toBe("/instances/new");
    expect(router.state.location.search).toMatchObject({ step: "source" });
    expect(
      invoke.mock.calls.some(
        ([command]) => command === "create_instance_with_platform",
      ),
    ).toBe(false);
  });

  it("opens settings from the Licht menu using the keyboard", async () => {
    mockContextMenuCommands((command) => {
      if (command === "runtime_java")
        return Promise.resolve({
          java25: null,
          java21: null,
          java17: null,
          java8: null,
        });
      return undefined;
    });
    await renderAt("/");
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeVisible();
    const trigger = await screen.findByRole("button", {
      name: "Licht Launcher",
    });
    trigger.focus();
    fireEvent.keyDown(trigger, { key: "Enter" });
    const menu = await screen.findByRole("menu", { name: "Licht menu" });
    const settings = within(menu).getByRole("menuitem", { name: "Settings" });
    const accounts = within(menu).getByRole("menuitem", {
      name: "Manage accounts",
    });
    await waitFor(() => expect(accounts).toHaveFocus());
    fireEvent.keyDown(accounts, { key: "ArrowDown" });
    await waitFor(() => expect(settings).toHaveFocus());
    fireEvent.keyDown(settings, { key: "Enter" });
    expect(
      await screen.findByRole("dialog", { name: "Settings" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Appearance", current: "page" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("menu", { name: "Licht menu" }),
    ).not.toBeInTheDocument();
    expect(closeWindow).not.toHaveBeenCalled();
  });

  it("requests normal window closure from Exit in the Licht menu", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeVisible();
    const trigger = await screen.findByRole("button", {
      name: "Licht Launcher",
    });
    // Avoid hitting jsdom's zero-sized resize handle at the origin.
    fireEvent.pointerDown(trigger, {
      button: 0,
      ctrlKey: false,
      pointerType: "mouse",
      clientX: 50,
      clientY: 20,
    });
    fireEvent.pointerUp(trigger, {
      button: 0,
      pointerType: "mouse",
      clientX: 50,
      clientY: 20,
    });
    const menu = await screen.findByRole("menu", { name: "Licht menu" });
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Exit" }));
    await waitFor(() => expect(closeWindow).toHaveBeenCalledOnce());
  });

  it("opens the feedback form from the titlebar menu and restores its closed state", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    const titlebar = screen.getByRole("banner");
    const trigger = within(titlebar).getByRole("button", {
      name: "Licht Launcher",
    });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    trigger.focus();
    fireEvent.keyDown(trigger, { key: "Enter" });
    const menu = await screen.findByRole("menu", { name: "Licht menu" });
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    fireEvent.click(
      within(menu).getByRole("menuitem", { name: "Send feedback" }),
    );
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("open_feedback"));
    await waitFor(() =>
      expect(trigger).toHaveAttribute("aria-expanded", "false"),
    );
  });

  it.each([
    {
      name: "OnlinePlayer",
      profile: {
        uuid: "0123456789abcdef0123456789abcdef",
        name: "OnlinePlayer",
        skinUrl: `https://textures.minecraft.net/texture/${"a".repeat(64)}`,
      },
    },
    { name: "Steve", profile: null },
  ])(
    "shows $name on the right side of the titlebar with a cached head and an account menu",
    async ({ name, profile }) => {
      mockContextMenuCommands((command) => {
        if (command === "active_microsoft_account")
          return Promise.resolve(profile);
        if (command === "list_microsoft_accounts")
          return Promise.resolve(profile ? [profile] : []);
        if (command === "runtime_java")
          return Promise.resolve({
            java25: null,
            java21: null,
            java17: null,
            java8: null,
          });
        return undefined;
      });
      await renderAt("/");
      const account = await screen.findByRole("button", {
        name: `Account: ${name}`,
      });
      const footer = screen.getByRole("contentinfo", {
        name: "Download status",
      });
      const titlebar = screen.getByRole("banner");
      expect(titlebar).toContainElement(account);
      expect(
        within(footer).queryByRole("button", {
          name: /^Account:/,
        }),
      ).not.toBeInTheDocument();
      expect(
        within(titlebar)
          .getByRole("button", { name: "Library" })
          .compareDocumentPosition(account) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
      expect(screen.getAllByRole("button", { name: /^Account:/ })).toHaveLength(
        1,
      );
      await waitFor(() =>
        expect(account.querySelector("img")).toHaveAttribute(
          "src",
          `asset://localhost/${playerHeadPathFixture}`,
        ),
      );
      expect(invoke).toHaveBeenCalledWith("get_player_head", {
        skinUrl: profile?.skinUrl ?? null,
      });
      expect(account).toHaveTextContent(name);
      account.focus();
      fireEvent.keyDown(account, { key: "Enter" });
      const menu = await screen.findByRole("menu", { name: "Player accounts" });
      expect(account).toHaveAttribute("aria-expanded", "true");
      expect(
        within(menu).getByRole("menuitem", { name: "Add account" }),
      ).toBeVisible();
      if (profile)
        expect(
          await within(menu).findByRole("menuitemradio", {
            name: profile.name,
          }),
        ).toHaveAttribute("aria-checked", "true");
      fireEvent.keyDown(menu, { key: "Escape" });
      await waitFor(() =>
        expect(
          screen.queryByRole("menu", { name: "Player accounts" }),
        ).not.toBeInTheDocument(),
      );
      expect(account).toHaveAttribute("aria-expanded", "false");
    },
  );

  it("opens Account settings from Add account in the titlebar without starting login", async () => {
    mockContextMenuCommands((command) => {
      if (command === "runtime_java")
        return Promise.resolve({
          java25: null,
          java21: null,
          java17: null,
          java8: null,
        });
      return undefined;
    });
    await renderAt("/");
    const account = await within(screen.getByRole("banner")).findByRole(
      "button",
      { name: "Account: Steve" },
    );
    account.focus();
    fireEvent.keyDown(account, { key: "Enter" });
    const menu = await screen.findByRole("menu", { name: "Player accounts" });
    fireEvent.click(
      within(menu).getByRole("menuitem", { name: "Add account" }),
    );
    const settings = await screen.findByRole("dialog", { name: "Settings" });
    expect(settings).toBeVisible();
    expect(
      within(settings).getByRole("button", {
        name: "Account",
        current: "page",
      }),
    ).toBeVisible();
    expect(invoke).not.toHaveBeenCalledWith("begin_microsoft_login");
    expect(
      screen.queryByRole("menu", { name: "Player accounts" }),
    ).not.toBeInTheDocument();
  });

  it.each([true, false])(
    "keeps sidebar actions and content usable with custom title bar %s",
    async (customTitleBar) => {
      mockContextMenuCommands((command) => {
        if (command === "custom_title_bar_active")
          return Promise.resolve(customTitleBar);
        if (command === "runtime_java") {
          return Promise.resolve({
            java25: null,
            java21: null,
            java17: null,
            java8: null,
          });
        }
        return undefined;
      });
      await renderAt("/");
      expect(
        await screen.findByRole("heading", { name: "Classic" }),
      ).toBeVisible();
      expect(
        screen.getByRole("navigation", { name: "Instances" }),
      ).toBeVisible();
      const footer = screen.getByRole("contentinfo", {
        name: "Download status",
      });
      expect(
        await within(footer).findByRole("button", { name: "Downloads: Idle" }),
      ).toBeVisible();

      const titleBar = screen.getByRole("banner");
      expect(
        within(titleBar).getByRole("button", { name: "Licht Launcher" }),
      ).toBeVisible();
      if (!customTitleBar) {
        expect(
          within(titleBar).queryByRole("button", { name: "Minimize" }),
        ).not.toBeInTheDocument();
        expect(
          within(titleBar).queryByRole("button", { name: "Close" }),
        ).not.toBeInTheDocument();
      }
      const launcher = within(titleBar).getByRole("button", {
        name: "Licht Launcher",
      });
      launcher.focus();
      fireEvent.keyDown(launcher, { key: "Enter" });
      fireEvent.click(
        within(
          await screen.findByRole("menu", { name: "Licht menu" }),
        ).getByRole("menuitem", { name: "Manage accounts" }),
      );
      const settings = await screen.findByRole("dialog", { name: "Settings" });
      expect(
        within(settings).getByRole("button", {
          name: "Account",
          current: "page",
        }),
      ).toBeVisible();
      fireEvent.click(within(settings).getByRole("button", { name: "Close" }));
      await waitFor(() =>
        expect(
          screen.queryByRole("dialog", { name: "Settings" }),
        ).not.toBeInTheDocument(),
      );
      expect(screen.getByRole("heading", { name: "Classic" })).toBeVisible();
    },
  );

  it("shows download progress in the footer and Downloads screen, then returns to idle", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    const footer = within(
      screen.getByRole("contentinfo", { name: "Download status" }),
    );
    await footer.findByRole("button", { name: "Downloads: Idle" });
    await waitFor(() =>
      expect(listeners.has("download-job-updated")).toBe(true),
    );
    act(() =>
      listeners.get("download-job-updated")?.({ payload: downloadJob() }),
    );
    const trigger = await footer.findByRole("button", {
      name: "Downloads: 25%",
    });
    expect(trigger).not.toHaveTextContent("Downloads");
    expect(within(trigger).getByText("Downloading · Minecraft")).toBeVisible();
    expect(
      within(trigger).getByRole("progressbar", {
        name: "Current download progress",
      }),
    ).toHaveAttribute("aria-valuenow", "25");
    fireEvent.click(trigger);
    expect(await screen.findByText(/25\.0%/)).toBeInTheDocument();
    act(() =>
      listeners.get("download-job-updated")?.({ payload: completedDownload() }),
    );
    expect(
      await footer.findByRole("button", { name: "Downloads: Idle" }),
    ).toBeInTheDocument();
  });

  it("opens idle download details from the footer", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    const footer = within(
      screen.getByRole("contentinfo", { name: "Download status" }),
    );
    const trigger = await footer.findByRole("button", {
      name: "Downloads: Idle",
    });
    expect(trigger).toHaveTextContent(/^Downloads$/);
    fireEvent.click(trigger);
    expect(
      await screen.findByText("No downloads in progress"),
    ).toBeInTheDocument();
    expect(
      within(screen.getByRole("banner")).queryByRole("button", {
        name: /^Downloads:/,
      }),
    ).not.toBeInTheDocument();
  });

  it("shows preparing downloads in the footer before totals are known", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    await waitFor(() =>
      expect(listeners.has("download-job-updated")).toBe(true),
    );
    act(() =>
      listeners.get("download-job-updated")?.({
        payload: downloadJob({
          component: "fabric",
          phase: "preparing",
          progress: {
            finished: 0,
            failed: 0,
            total: 0,
            bytesDone: 0,
            bytesTotal: 0,
          },
        }),
      }),
    );
    const footer = within(
      screen.getByRole("contentinfo", { name: "Download status" }),
    );
    expect(
      await footer.findByRole("button", { name: "Downloads: Preparing" }),
    ).toBeInTheDocument();
  });

  it("aggregates multiple installs in the footer and opens both download rows", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    await waitFor(() =>
      expect(listeners.has("download-job-updated")).toBe(true),
    );
    act(() => {
      listeners.get("download-job-updated")?.({
        payload: downloadJob({ instanceFolder: "First" }),
      });
      listeners.get("download-job-updated")?.({
        payload: downloadJob({ id: "2", instanceFolder: "Second" }),
      });
    });
    const footer = within(
      screen.getByRole("contentinfo", { name: "Download status" }),
    );
    fireEvent.click(
      await footer.findByRole("button", { name: "Downloads: 2" }),
    );
    expect(
      await screen.findByRole("region", { name: "Download for First" }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("region", { name: "Download for Second" }),
    ).toBeInTheDocument();
  });

  it.each(["paused", "interrupted", "failed"] as const)(
    "offers an explicit resume with appropriate styling for a %s download",
    async (status) => {
      let job = downloadJob({ instanceFolder: "classic", status });
      mockContextMenuCommands((command) => {
        if (command === "list_download_jobs") return Promise.resolve([job]);
        if (command === "resume_download_job") {
          job = { ...job, status: "running" };
          return Promise.resolve(job);
        }
      });
      await renderAt("/?instance=classic");
      const resume = await screen.findByRole("button", {
        name: "Resume download for Classic",
      });
      expect(resume).toHaveAttribute(
        "data-variant",
        status === "paused" ? "secondary" : "warning",
      );
      expect(invoke).not.toHaveBeenCalledWith("resume_download_job", {
        jobId: "1",
      });
      fireEvent.click(resume);
      await screen.findByRole("button", { name: "Pause download for Classic" });
      expect(invoke).toHaveBeenCalledWith("resume_download_job", {
        jobId: "1",
      });
      expect(invoke.mock.calls.some(([command]) => command === "launch")).toBe(
        false,
      );
    },
  );

  it("pauses a download on the instance page and preserves a repair after confirmed cancellation", async () => {
    let job = downloadJob({ instanceFolder: "classic", operation: "repair" });
    let cancelled = false;
    mockContextMenuCommands((command) => {
      if (command === "list_download_jobs")
        return Promise.resolve(cancelled ? [] : [job]);
      if (command === "pause_download_job") {
        job = { ...job, status: "paused" };
        return Promise.resolve(job);
      }
      if (command === "cancel_download_job") {
        cancelled = true;
        return Promise.resolve({ ...job, status: "cancelled" });
      }
    });
    await renderAt("/?instance=classic");
    fireEvent.click(
      await screen.findByRole("button", { name: "Pause download for Classic" }),
    );
    await screen.findByRole("button", { name: "Resume download for Classic" });
    fireEvent.click(
      screen.getByRole("button", { name: "Cancel download for Classic" }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Cancel download for Classic?",
    });
    expect(dialog).toHaveTextContent(
      "Existing instances and their files will be kept.",
    );
    expect(invoke).not.toHaveBeenCalledWith("cancel_download_job", {
      jobId: "1",
    });
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Cancel download" }),
    );
    await screen.findByRole("button", { name: "Play" });
    const sidebar = screen.getByRole("navigation", { name: "Instances" });
    expect(
      within(sidebar).getByRole("button", { name: /^Classic/ }),
    ).toBeVisible();
    expect(
      invoke.mock.calls.some(([command]) => command === "delete_instance"),
    ).toBe(false);
  });

  it("retries persisted cancellation from the instance page without resuming installation", async () => {
    let jobs = [
      downloadJob({
        instanceFolder: "classic",
        status: "cancelling",
        error: "Instance file is locked",
      }),
    ];
    let entries: InstanceEntry[] = [
      {
        ...classicInstance,
        installation: {
          source: { type: "vanilla", gameVersion: "1.5.2" },
          state: "installing",
        },
      },
      contextInstance,
    ];
    mockContextMenuCommands((command) => {
      if (command === "list_instances") return Promise.resolve(entries);
      if (command === "list_download_jobs") return Promise.resolve(jobs);
      if (command === "cancel_download_job") {
        const job = jobs[0];
        jobs = [];
        entries = entries.filter((entry) => entry.folder !== "classic");
        return Promise.resolve({
          ...job,
          status: "cancelled",
          error: undefined,
        });
      }
    });
    await renderAt("/?instance=classic");
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Retry cancellation for Classic",
      }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Retry cancellation for Classic?",
    });
    expect(within(dialog).getByRole("alert")).toHaveTextContent(
      "Instance file is locked",
    );
    expect(
      screen.queryByRole("button", { name: "Resume download for Classic" }),
    ).not.toBeInTheDocument();
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Retry cancellation" }),
    );
    await screen.findByRole("heading", { name: "Next" });
    expect(invoke).toHaveBeenCalledWith("cancel_download_job", { jobId: "1" });
    expect(
      invoke.mock.calls.some(
        ([command]) =>
          command === "resume_download_job" ||
          command === "prepare_instance_download" ||
          command === "launch",
      ),
    ).toBe(false);
  });

  it.each(["instance page", "downloads screen"])(
    "removes a new instance only after confirming cancellation from the %s",
    async (location) => {
      let entries: InstanceEntry[] = [
        {
          ...classicInstance,
          installation: {
            source: { type: "vanilla", gameVersion: "1.5.2" },
            state: "installing",
          },
        },
        contextInstance,
      ];
      let jobs = [
        downloadJob({ instanceFolder: "classic", versionId: "1.5.2" }),
      ];
      mockContextMenuCommands((command) => {
        if (command === "list_instances") return Promise.resolve(entries);
        if (command === "list_download_jobs") return Promise.resolve(jobs);
        if (command === "cancel_download_job") {
          const job = jobs[0];
          jobs = [];
          entries = entries.filter((entry) => entry.folder !== "classic");
          return Promise.resolve({ ...job, status: "cancelled" });
        }
      });
      await renderAt("/?instance=classic");
      if (location === "downloads screen") {
        fireEvent.click(
          await screen.findByRole("button", { name: "Downloads: 25%" }),
        );
        const row = await screen.findByRole("region", {
          name: "Download for Classic",
        });
        fireEvent.click(
          within(row).getByRole("button", {
            name: "Cancel download for Classic",
          }),
        );
      } else {
        fireEvent.click(
          await screen.findByRole("button", {
            name: "Cancel download for Classic",
          }),
        );
      }
      const dialog = await screen.findByRole("dialog", {
        name: "Cancel download for Classic?",
      });
      expect(dialog).toHaveTextContent(
        "removed from your library, along with its files",
      );
      expect(invoke).not.toHaveBeenCalledWith("cancel_download_job", {
        jobId: "1",
      });
      const before = screen.getByLabelText(
        location === "downloads screen" ? "Download for Classic" : "Instances",
      );
      expect(within(before).getByText("Classic")).toBeInTheDocument();
      fireEvent.click(
        within(dialog).getByRole("button", { name: "Cancel download" }),
      );
      await waitFor(() =>
        expect(
          screen.queryByRole("dialog", {
            name: "Cancel download for Classic?",
          }),
        ).not.toBeInTheDocument(),
      );
      if (location === "downloads screen") {
        fireEvent.click(screen.getByRole("button", { name: "Library" }));
      }
      const sidebar = await screen.findByRole("navigation", {
        name: "Instances",
      });
      await waitFor(() =>
        expect(within(sidebar).queryByText("Classic")).not.toBeInTheDocument(),
      );
      expect(within(sidebar).getByText("Next")).toBeInTheDocument();
      expect(invoke).toHaveBeenCalledWith("cancel_download_job", {
        jobId: "1",
      });
    },
  );

  it("sorts instances by last played or name", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([
          {
            ...classicInstance,
            folder: "zeta",
            name: "Zeta",
            lastOpenedAt: "2026-01-01T10:00:00.000Z",
          },
          {
            ...classicInstance,
            lastOpenedAt: "2026-03-01T10:00:00.000Z",
          },
          {
            ...classicInstance,
            folder: "alpha",
            name: "Alpha",
            lastOpenedAt: null,
          },
        ]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.5.2", versionType: "release", installed: true },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "get_last_played_folder") {
        return Promise.resolve("classic");
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");

    const sidebar = await screen.findByRole("navigation", {
      name: "Instances",
    });
    const names = () =>
      within(sidebar)
        .getAllByRole("button")
        .map((button) => button.textContent)
        .filter(
          (text) => text === "Alpha" || text === "Classic" || text === "Zeta",
        );

    await waitFor(() => {
      expect(names()).toEqual(["Classic", "Zeta", "Alpha"]);
    });
    expect(
      screen.getByRole("separator", { name: "Resize sidebar" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("combobox", { name: "Sort instances" }));
    expect(await screen.findByRole("listbox")).toHaveAttribute(
      "data-side",
      "bottom",
    );
    fireEvent.click(await screen.findByRole("option", { name: "A–Z" }));
    expect(names()).toEqual(["Alpha", "Classic", "Zeta"]);

    fireEvent.click(screen.getByRole("combobox", { name: "Sort instances" }));
    fireEvent.click(await screen.findByRole("option", { name: "Z–A" }));
    expect(names()).toEqual(["Zeta", "Classic", "Alpha"]);

    fireEvent.click(screen.getByRole("combobox", { name: "Sort instances" }));
    expect(await screen.findByRole("listbox")).toHaveAttribute(
      "data-side",
      "bottom",
    );
    fireEvent.click(await screen.findByRole("option", { name: "Last played" }));
    expect(names()).toEqual(["Classic", "Zeta", "Alpha"]);
  });

  it("keeps the empty sidebar clear and opens settings", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: false },
          { id: "1.5.2", versionType: "release", installed: true },
          { id: "24w14a", versionType: "snapshot", installed: false },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "runtime_java") {
        return Promise.resolve({
          java25: null,
          java21: null,
          java17: null,
          java8: null,
        });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "get_last_played_folder") {
        return Promise.resolve(null);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");

    expect(
      await screen.findByRole("heading", {
        name: "Create your first instance",
      }),
    ).toBeInTheDocument();
    expect(screen.queryByText("No instances yet.")).not.toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "Create instance" }),
    ).toHaveAttribute("href");
    const appMenu = screen.getByRole("button", { name: "Licht Launcher" });
    expect(appMenu).toBeVisible();
    expect(appMenu).toHaveAttribute("aria-haspopup", "menu");
    expect(appMenu.closest("[data-tauri-drag-region]")).toBeNull();
    const mainNavigation = within(
      screen.getByRole("navigation", { name: "Main navigation" }),
    );
    expect(
      mainNavigation.getByRole("button", { name: "Library", current: "page" }),
    ).toBeEnabled();
    expect(
      mainNavigation.getByRole("button", { name: "Discover" }),
    ).toBeDisabled();
    expect(appMenu.closest("[data-panel]")).toBeNull();
    expect(screen.getByRole("banner")).toContainElement(appMenu);
    expect(
      screen.queryByRole("group", { name: "Sidebar actions" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Skins" }),
    ).not.toBeInTheDocument();
    const sidebar = within(
      screen.getByRole("navigation", { name: "Instances" }),
    );
    expect(
      sidebar.queryByRole("button", { name: "New instance" }),
    ).not.toBeInTheDocument();
    expect(
      sidebar.getByRole("combobox", { name: "Sort instances" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("list", { name: "Versions" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("combobox", { name: "Type" }),
    ).not.toBeInTheDocument();

    appMenu.focus();
    fireEvent.keyDown(appMenu, { key: "Enter" });
    fireEvent.click(
      within(await screen.findByRole("menu", { name: "Licht menu" })).getByRole(
        "menuitem",
        { name: "Manage accounts" },
      ),
    );
    expect(
      await screen.findByRole("button", {
        name: "Account",
        current: "page",
      }),
    ).toBeInTheDocument();
    fireEvent.click(
      within(screen.getByRole("dialog", { name: "Settings" })).getByRole(
        "button",
        { name: "Close" },
      ),
    );
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "Settings" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("replaces instance tabs with a welcome state and opens the creation wizard", async () => {
    mockContextMenuCommands((command) => {
      if (command === "list_instances") return Promise.resolve([]);
    });

    const { router } = await renderAt("/");

    expect(
      await screen.findByRole("heading", {
        name: "Create your first instance",
      }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Choose a Minecraft version or install a modpack to start playing.",
      ),
    ).toBeVisible();
    expect(screen.queryByRole("tablist")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("textbox", { name: "Search content" }),
    ).not.toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Create instance" }));
      await router.load();
    });

    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeVisible();
  });

  it("waits for the instance list before showing the welcome state", async () => {
    let finishLoading: (entries: InstanceEntry[]) => void = () => {};
    const pendingInstances = new Promise<InstanceEntry[]>((resolve) => {
      finishLoading = resolve;
    });
    mockContextMenuCommands((command) => {
      if (command === "list_instances") return pendingInstances;
    });

    await renderAt("/");

    expect(
      screen.queryByRole("heading", { name: "Create your first instance" }),
    ).not.toBeInTheDocument();

    await act(async () => finishLoading([]));

    expect(
      await screen.findByRole("heading", {
        name: "Create your first instance",
      }),
    ).toBeVisible();
  });

  it("lists instances in the sidebar and plays an installed version", async () => {
    let finishLaunch: (exit: { code: number }) => void = () => {};
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([
          {
            ...classicInstance,
            lastOpenedAt: new Date(Date.now() - 5 * 60_000).toISOString(),
          },
        ]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.5.2", versionType: "release", installed: true },
          { id: "24w14a", versionType: "snapshot", installed: false },
        ]);
      }
      if (command === "is_version_ready_cmd") {
        return Promise.resolve(true);
      }
      if (command === "get_settings") {
        return Promise.resolve({
          ...snapshot,
          settings: { ...snapshot.settings, username: "Mateus" },
        });
      }
      if (command === "touch_last_opened") {
        return Promise.resolve({
          ...classicInstance,
          lastOpenedAt: "2026-10-11T15:00:00Z",
        });
      }
      if (command === "launch") {
        return new Promise<{ code: number }>((resolve) => {
          finishLaunch = resolve;
        });
      }
      if (command === "get_last_played_folder") {
        return Promise.resolve("classic");
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "stop_game") {
        return Promise.resolve(undefined);
      }
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
      if (command === "custom_title_bar_active") {
        return Promise.resolve(true);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");

    const sidebar = await screen.findByRole("navigation", {
      name: "Instances",
    });
    expect(within(sidebar).getByText("Classic")).toBeInTheDocument();
    expect(within(sidebar).getByText("Classic")).toHaveClass(
      "overflow-hidden",
      "text-ellipsis",
      "whitespace-nowrap",
    );
    expect(within(sidebar).queryByText("1.5.2")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Username")).not.toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Classic" }),
    ).toBeInTheDocument();
    expect(screen.getByText("1.5.2")).toBeInTheDocument();
    expect(screen.getByText("Vanilla")).toBeInTheDocument();
    expect(screen.getByText("5 minutes ago")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Configure Classic" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Abrir Pasta" }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Play" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "classic",
        username: "Mateus",
      });
    });
    expect(screen.getByRole("button", { name: "Starting…" })).toBeDisabled();
    expect(
      screen.queryByRole("button", { name: "Stop" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Configure Classic" }),
    ).toBeEnabled();
    expect(
      screen.getByRole("link", { name: "Create instance" }),
    ).toHaveAttribute("href");
    expect(
      screen.getByRole("button", { name: "Licht Launcher" }),
    ).toBeEnabled();
    expect(screen.queryByText("Preparing…")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("progressbar", { name: "Install progress" }),
    ).not.toBeInTheDocument();

    listeners.get("game-log")?.({
      payload: {
        folder: "classic",
        stream: "stdout",
        line: "Sound engine started",
      } satisfies GameLogEvent,
    });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();
    });
    expect(
      screen.queryByRole("button", { name: "Starting…" }),
    ).not.toBeInTheDocument();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Console" }));
    expect(
      await screen.findByRole("button", { name: "Log level" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Sound engine started")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Files" })).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith(
      "prepare_instance_download",
      expect.anything(),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "install_modpack_files",
      expect.anything(),
    );

    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    expect(
      await screen.findByRole("button", { name: "Stopping…" }),
    ).toBeDisabled();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("stop_game", { folder: "classic" });
    });
    finishLaunch({ code: 0 });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Play" })).toBeEnabled();
    });
    expect(
      screen.queryByRole("button", { name: "Stopping…" }),
    ).not.toBeInTheDocument();
  });

  it("shows Stop for an orphaned running game recovered on load", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([classicInstance]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.5.2", versionType: "release", installed: true },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "list_running_games") {
        return Promise.resolve(["classic"]);
      }
      if (command === "stop_game") {
        return Promise.resolve(undefined);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");
    expect(await screen.findByRole("button", { name: "Stop" })).toBeEnabled();
    expect(
      screen.queryByRole("button", { name: "Play" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Classic (running)" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("stop_game", { folder: "classic" });
    });
    expect(await screen.findByRole("button", { name: "Play" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Classic" })).toBeInTheDocument();
  });

  it("keeps a console per instance and allows parallel launches", async () => {
    const finishLaunch: Record<string, (exit: { code: number }) => void> = {};
    const nextInstance = {
      ...classicInstance,
      folder: "next",
      name: "Next",
      versionId: "1.20.1",
    };
    invoke.mockImplementation((command: string, args?: { folder?: string }) => {
      if (command === "list_instances") {
        return Promise.resolve([classicInstance, nextInstance]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.5.2", versionType: "release", installed: true },
          { id: "1.20.1", versionType: "release", installed: true },
        ]);
      }
      if (command === "is_version_ready_cmd") {
        return Promise.resolve(true);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "touch_last_opened") {
        return Promise.resolve({
          ...classicInstance,
          folder: args?.folder ?? "classic",
          lastOpenedAt: "2026-10-11T15:00:00Z",
        });
      }
      if (command === "launch" && args?.folder) {
        return new Promise<{ code: number }>((resolve) => {
          finishLaunch[args.folder as string] = resolve;
        });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
      if (command === "custom_title_bar_active") {
        return Promise.resolve(true);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");
    const sidebar = await screen.findByRole("navigation", {
      name: "Instances",
    });

    fireEvent.click(screen.getByRole("button", { name: "Play" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "classic",
        username: "Steve",
      });
    });
    expect(screen.getByRole("button", { name: "Starting…" })).toBeDisabled();

    fireEvent.click(within(sidebar).getByRole("button", { name: /^Next/ }));
    expect(await screen.findByRole("button", { name: "Play" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Play" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "next",
        username: "Steve",
      });
    });
    expect(screen.getByRole("button", { name: "Starting…" })).toBeDisabled();

    listeners.get("game-log")?.({
      payload: {
        folder: "classic",
        stream: "stdout",
        line: "classic-log",
      } satisfies GameLogEvent,
    });
    listeners.get("game-log")?.({
      payload: {
        folder: "next",
        stream: "stdout",
        line: "next-log",
      } satisfies GameLogEvent,
    });

    fireEvent.mouseDown(screen.getByRole("tab", { name: "Console" }));
    expect(
      await screen.findByRole("button", { name: "Log level" }),
    ).toBeInTheDocument();
    expect(screen.getByText("next-log")).toBeInTheDocument();
    expect(screen.queryByText("classic-log")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Clear console" }));
    expect(screen.queryByText("next-log")).not.toBeInTheDocument();

    fireEvent.click(within(sidebar).getByRole("button", { name: /^Classic/ }));
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Console" }));
    expect(
      await screen.findByRole("button", { name: "Log level" }),
    ).toBeInTheDocument();
    expect(screen.getByText("classic-log")).toBeInTheDocument();
    expect(screen.queryByText("next-log")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();

    finishLaunch.classic?.({ code: 0 });
    finishLaunch.next?.({ code: 0 });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Play" })).toBeEnabled();
    });
  });

  it("installs a missing version when playing", async () => {
    let finishInstall: (job: DownloadJobSnapshot) => void = () => {};
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([
          {
            ...classicInstance,
            versionId: "1.20.1",
            folder: "next",
            name: "Next",
          },
        ]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: false },
        ]);
      }
      if (command === "is_version_ready_cmd") {
        return Promise.resolve(false);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "prepare_instance_download") {
        return new Promise<DownloadJobSnapshot>((resolve) => {
          finishInstall = resolve;
        });
      }
      if (command === "install_modpack_files") {
        return Promise.resolve();
      }
      if (command === "touch_last_opened") {
        return Promise.resolve({
          ...classicInstance,
          versionId: "1.20.1",
          lastOpenedAt: "2026-10-11T15:00:00Z",
        });
      }
      if (command === "launch") {
        return Promise.resolve({ code: 0 });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
        folder: "next",
        operation: "install",
      });
    });
    expect(
      await screen.findByRole("progressbar", { name: "Install progress" }),
    ).toBeInTheDocument();

    const menu = await openInstanceContextMenu();
    expect(
      within(menu).getByRole("menuitem", { name: "Installing…" }),
    ).toHaveAttribute("aria-disabled", "true");
    expect(
      within(menu).getByRole("menuitem", { name: "Duplicate" }),
    ).toHaveAttribute("aria-disabled", "true");
    fireEvent.keyDown(menu, { key: "Escape" });
    await waitFor(() => {
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    listeners.get("install-progress")?.({
      payload: {
        versionId: "1.20.1",
        finished: 1,
        failed: 0,
        total: 4,
        bytesDone: 5_000_000,
        bytesTotal: 10_000_000,
      } satisfies InstallProgress,
    });
    expect(
      await screen.findByText(/50\.0%\s+5\.0MB\/10\.0MB/),
    ).toBeInTheDocument();

    finishInstall(completedDownload({ instanceFolder: "next" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "next",
        username: "Steve",
      });
    });
  });

  it("skips prepare when the version is already ready", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([
          {
            ...classicInstance,
            versionId: "fabric-loader-0.16.0-1.20.1",
            folder: "modded",
            name: "Modded",
          },
        ]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: true },
          {
            id: "fabric-loader-0.16.0-1.20.1",
            versionType: "old_alpha",
            installed: true,
          },
        ]);
      }
      if (command === "is_version_ready_cmd") {
        return Promise.resolve(true);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "touch_last_opened") {
        return Promise.resolve({
          ...classicInstance,
          folder: "modded",
          name: "Modded",
          versionId: "fabric-loader-0.16.0-1.20.1",
          lastOpenedAt: "2026-10-11T15:00:00Z",
        });
      }
      if (command === "launch") {
        return Promise.resolve({ code: 0 });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "modded",
        username: "Steve",
      });
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "prepare_instance_download",
      expect.anything(),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "install_modpack_files",
      expect.anything(),
    );
  });

  it("keeps Preparing while install progress has no totals yet", async () => {
    let finishInstall: (job: DownloadJobSnapshot) => void = () => {};
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([
          {
            ...classicInstance,
            versionId: "fabric-loader-0.16.0-1.20.1",
            folder: "modded",
            name: "Modded",
          },
        ]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          {
            id: "fabric-loader-0.16.0-1.20.1",
            versionType: "old_alpha",
            installed: false,
          },
        ]);
      }
      if (command === "is_version_ready_cmd") {
        return Promise.resolve(false);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "prepare_instance_download") {
        return new Promise<DownloadJobSnapshot>((resolve) => {
          finishInstall = resolve;
        });
      }
      if (command === "install_modpack_files") {
        return Promise.resolve();
      }
      if (command === "touch_last_opened") {
        return Promise.resolve({
          ...classicInstance,
          folder: "modded",
          name: "Modded",
          versionId: "fabric-loader-0.16.0-1.20.1",
          lastOpenedAt: "2026-10-11T15:00:00Z",
        });
      }
      if (command === "launch") {
        return Promise.resolve({ code: 0 });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));

    expect(
      await screen.findByRole("progressbar", { name: "Install progress" }),
    ).toBeInTheDocument();

    listeners.get("install-progress")?.({
      payload: {
        versionId: "fabric-loader-0.16.0-1.20.1",
        finished: 0,
        failed: 0,
        total: 0,
        bytesDone: 0,
        bytesTotal: 0,
      } satisfies InstallProgress,
    });
    expect(await screen.findByText("Preparing…")).toBeInTheDocument();

    listeners.get("install-progress")?.({
      payload: {
        versionId: "fabric-loader-0.16.0-1.20.1",
        finished: 1,
        failed: 0,
        total: 4,
        bytesDone: 5_000_000,
        bytesTotal: 10_000_000,
      } satisfies InstallProgress,
    });
    expect(
      await screen.findByText(/50\.0%\s+5\.0MB\/10\.0MB/),
    ).toBeInTheDocument();

    finishInstall(
      completedDownload({
        instanceFolder: "modded",
        versionId: "fabric-loader-0.16.0-1.20.1",
      }),
    );
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "modded",
        username: "Steve",
      });
    });
  });

  it("does not launch the game when preparation is cancelled", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances")
        return Promise.resolve([classicInstance]);
      if (command === "get_settings") return Promise.resolve(snapshot);
      if (command === "list_versions") return Promise.resolve([]);
      if (command === "list_running_games") return Promise.resolve([]);
      if (command === "is_version_ready_cmd") return Promise.resolve(false);
      if (command === "prepare_instance_download")
        return Promise.resolve(
          downloadJob({ status: "cancelled", instanceFolder: "classic" }),
        );
      if (command === "launch") return Promise.resolve({ code: 0 });
      return defaultCommandResponse(command);
    });
    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
        folder: "classic",
        operation: "install",
      }),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Play" })).toBeEnabled(),
    );
    expect(invoke).not.toHaveBeenCalledWith("launch", expect.anything());
  });

  it("prepares a pack with an unresolved version before enabling an explicit launch", async () => {
    let ready = false;
    let finish: (() => void) | undefined;
    const pending: InstanceEntry = {
      ...classicInstance,
      name: "Pending pack",
      versionId: null,
      installation: {
        state: "preparing",
        source: {
          type: "modpack",
          source: "modrinth",
          projectId: "pack",
          versionId: "pack-version",
          iconUrl: null,
        },
      },
    };
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances")
        return Promise.resolve([
          ready
            ? {
                ...pending,
                versionId: "fabric-loader-0.16.10-1.21.1",
                installation: { ...pending.installation, state: "ready" },
              }
            : pending,
        ]);
      if (command === "get_settings") return Promise.resolve(snapshot);
      if (command === "list_versions" || command === "list_running_games")
        return Promise.resolve([]);
      if (command === "is_version_ready_cmd") return Promise.resolve(ready);
      if (command === "prepare_instance_download")
        return new Promise<DownloadJobSnapshot>((resolve) => {
          finish = () => {
            ready = true;
            resolve(
              completedDownload({
                instanceFolder: "classic",
                versionId: "fabric-loader-0.16.10-1.21.1",
              }),
            );
          };
        });
      if (command === "touch_last_opened")
        return Promise.resolve({
          ...pending,
          versionId: "fabric-loader-0.16.10-1.21.1",
        });
      if (command === "launch") return Promise.resolve({ code: 0 });
      return defaultCommandResponse(command);
    });
    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Install" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
        folder: "classic",
        operation: "install",
      }),
    );
    expect(screen.getByRole("button", { name: "Install" })).toBeDisabled();
    expect(invoke).not.toHaveBeenCalledWith("launch", expect.anything());
    if (!finish) throw new Error("preparation was not started");
    await act(async () => finish?.());
    const play = await screen.findByRole("button", { name: "Play" });
    expect(invoke).not.toHaveBeenCalledWith("launch", expect.anything());
    fireEvent.click(play);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "classic",
        username: "Steve",
      }),
    );
  });

  it("submits two installation requests without locking the sidebar", async () => {
    const finishInstall: Record<string, (job: DownloadJobSnapshot) => void> =
      {};
    invoke.mockImplementation((command: string, args?: unknown) => {
      if (command === "list_instances") {
        return Promise.resolve([
          {
            ...classicInstance,
            versionId: "1.20.1",
            folder: "next",
            name: "Next",
          },
          {
            ...classicInstance,
            versionId: "1.21.1",
            folder: "fresh",
            name: "Fresh",
          },
        ]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: false },
          { id: "1.21.1", versionType: "release", installed: false },
        ]);
      }
      if (command === "is_version_ready_cmd") {
        return Promise.resolve(false);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "prepare_instance_download") {
        const folder = (args as { folder: string }).folder;
        const versionId = folder === "next" ? "1.20.1" : "1.21.1";
        return new Promise<DownloadJobSnapshot>((resolve) => {
          finishInstall[versionId] = resolve;
        });
      }
      if (command === "install_modpack_files") {
        return Promise.resolve();
      }
      if (command === "touch_last_opened") {
        return Promise.resolve({
          ...classicInstance,
          folder: (args as { folder?: string } | undefined)?.folder ?? "fresh",
          lastOpenedAt: "2026-10-11T15:00:00Z",
        });
      }
      if (command === "launch") {
        return Promise.resolve({ code: 0 });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");
    const sidebar = screen.getByRole("navigation", { name: "Instances" });
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
        folder: "next",
        operation: "install",
      });
    });
    expect(
      screen.getByRole("link", { name: "Create instance" }),
    ).toHaveAttribute("href");
    expect(
      within(sidebar).getByRole("button", { name: /^Fresh/ }),
    ).toBeEnabled();

    fireEvent.click(within(sidebar).getByRole("button", { name: /^Fresh/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("prepare_instance_download", {
        folder: "fresh",
        operation: "install",
      });
    });

    listeners.get("install-progress")?.({
      payload: {
        versionId: "1.21.1",
        finished: 1,
        failed: 0,
        total: 2,
        bytesDone: 4_000_000,
        bytesTotal: 8_000_000,
      } satisfies InstallProgress,
    });
    expect(
      await screen.findByText(/50\.0%\s+4\.0MB\/8\.0MB/),
    ).toBeInTheDocument();

    fireEvent.click(within(sidebar).getByRole("button", { name: /^Next/ }));
    listeners.get("install-progress")?.({
      payload: {
        versionId: "1.20.1",
        finished: 1,
        failed: 0,
        total: 4,
        bytesDone: 5_000_000,
        bytesTotal: 10_000_000,
      } satisfies InstallProgress,
    });
    expect(
      await screen.findByText(/50\.0%\s+5\.0MB\/10\.0MB/),
    ).toBeInTheDocument();

    finishInstall["1.20.1"]?.(completedDownload({ instanceFolder: "next" }));
    finishInstall["1.21.1"]?.(
      completedDownload({ instanceFolder: "fresh", versionId: "1.21.1" }),
    );
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "next",
        username: "Steve",
      });
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "fresh",
        username: "Steve",
      });
    });
  });

  it("opens the new instance wizard from the footer", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: false },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");
    const createAction = await screen.findByRole("link", {
      name: "Create instance",
    });
    expect(
      screen.getByRole("button", { name: "Library", current: "page" }),
    ).toBeEnabled();
    fireEvent.click(createAction);
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Library" })).not.toHaveAttribute(
      "aria-current",
    );
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Library" })).toHaveAttribute(
        "aria-current",
        "page",
      ),
    );
  });

  it("returns to the selected instance through Library and preserves history", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/?instance=next");
    expect(await screen.findByRole("heading", { name: "Next" })).toBeVisible();
    fireEvent.click(screen.getByRole("link", { name: "Create instance" }));
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeVisible();
    const library = screen.getByRole("button", { name: "Library" });
    expect(library).not.toHaveAttribute("aria-current");
    fireEvent.click(library);
    expect(await screen.findByRole("heading", { name: "Next" })).toBeVisible();
    expect(library).toHaveAttribute("aria-current", "page");
    expect(router.state.location.search).toMatchObject({ instance: "next" });
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Forward" }));
    expect(await screen.findByRole("heading", { name: "Next" })).toBeVisible();
  });

  it("navigates back and forward from the sidebar history buttons", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: false },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "custom_title_bar_active") {
        return Promise.resolve(true);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");

    const back = await screen.findByRole("button", { name: "Back" });
    const forward = screen.getByRole("button", { name: "Forward" });
    expect(back).toBeDisabled();
    expect(forward).toBeDisabled();

    fireEvent.click(screen.getByRole("link", { name: "Create instance" }));
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Forward" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(
      await screen.findByRole("heading", {
        name: "Create your first instance",
      }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Forward" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Forward" }));
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Forward" })).toBeDisabled();
  });

  it("treats instance switches as history entries", async () => {
    const nextInstance = {
      ...classicInstance,
      folder: "next",
      name: "Next",
      versionId: "1.20.1",
    };
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([classicInstance, nextInstance]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.5.2", versionType: "release", installed: true },
          { id: "1.20.1", versionType: "release", installed: true },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "custom_title_bar_active") {
        return Promise.resolve(true);
      }
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
      return defaultCommandResponse(command);
    });

    await renderAt("/");
    const sidebar = await screen.findByRole("navigation", {
      name: "Instances",
    });
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();

    fireEvent.click(within(sidebar).getByRole("button", { name: /^Next/ }));
    expect(
      await screen.findByRole("heading", { name: "Next" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Forward" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Forward" }));
    expect(
      await screen.findByRole("heading", { name: "Next" }),
    ).toBeInTheDocument();
  });

  it.each([
    { instance: "next", folder: "next", name: "Next" },
    { instance: "missing", folder: "classic", name: "Classic" },
  ])("selects $folder when opening ?instance=$instance", async (entry) => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([
          classicInstance,
          { ...classicInstance, folder: "next", name: "Next" },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return defaultCommandResponse(command);
    });

    const { router } = await renderAt(`/?instance=${entry.instance}`);
    expect(
      await screen.findByRole("heading", { name: entry.name }),
    ).toBeInTheDocument();
    await waitFor(() => {
      expect(router.state.location.search).toMatchObject({
        instance: entry.folder,
      });
    });
    const sidebar = screen.getByRole("navigation", { name: "Instances" });
    expect(
      within(sidebar).getByRole("button", { name: entry.name }),
    ).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
  });

  describe("instance context menu", () => {
    beforeEach(() => {
      mockContextMenuCommands();
    });

    it("targets the clicked instance without changing selection or opening the global menu", async () => {
      const { router } = await renderAt("/?instance=classic");
      const menu = await openInstanceContextMenu();
      expect(screen.getAllByRole("menu")).toHaveLength(1);
      expect(within(menu).queryByText("Next")).not.toBeInTheDocument();
      expect(
        within(menu).queryByRole("menuitem", { name: "Settings" }),
      ).not.toBeInTheDocument();
      expect(router.state.location.search).toMatchObject({
        instance: "classic",
      });
      fireEvent.keyDown(menu, { key: "Escape" });
      await waitFor(() => {
        expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      });
      expect(screen.getByRole("heading", { name: "Classic" })).toBeVisible();
    });

    it("opens from the keyboard and restores focus when dismissed", async () => {
      await renderAt("/?instance=classic");
      const sidebar = await screen.findByRole("navigation", {
        name: "Instances",
      });
      const target = within(sidebar).getByRole("button", { name: "Next" });
      target.focus();
      fireEvent.keyDown(target, { key: "F10", shiftKey: true });
      const menu = await screen.findByRole("menu", {
        name: "Actions for Next",
      });
      expect(screen.getAllByRole("menu")).toHaveLength(1);
      fireEvent.keyDown(menu, { key: "Escape" });
      await waitFor(() => expect(target).toHaveFocus());
    });

    it("plays and stops the clicked instance and reflects transitional states", async () => {
      let finishLaunch: (exit: { code: number }) => void = () => {};
      let finishStop: () => void = () => {};
      mockContextMenuCommands((command) => {
        if (command === "launch") {
          return new Promise<{ code: number }>((resolve) => {
            finishLaunch = resolve;
          });
        }
        if (command === "stop_game") {
          return new Promise<void>((resolve) => {
            finishStop = resolve;
          });
        }
        return undefined;
      });
      await renderAt("/?instance=classic");
      await openInstanceContextMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "Play" }));
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("launch", {
          folder: "next",
          username: "Steve",
        });
      });
      let menu = await openInstanceContextMenu();
      expect(
        within(menu).getByRole("menuitem", { name: "Starting…" }),
      ).toHaveAttribute("aria-disabled", "true");
      expect(
        within(menu).getByRole("menuitem", { name: "Delete" }),
      ).toHaveAttribute("aria-disabled", "true");
      fireEvent.keyDown(menu, { key: "Escape" });
      act(() => {
        listeners.get("game-log")?.({
          payload: {
            folder: "next",
            stream: "stdout",
            line: "Started",
          } satisfies GameLogEvent,
        });
      });
      menu = await openInstanceContextMenu();
      fireEvent.click(within(menu).getByRole("menuitem", { name: "Stop" }));
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("stop_game", { folder: "next" });
      });
      menu = await openInstanceContextMenu();
      expect(
        within(menu).getByRole("menuitem", { name: "Stopping…" }),
      ).toHaveAttribute("aria-disabled", "true");
      await act(async () => {
        finishStop();
        finishLaunch({ code: 0 });
      });
      await waitFor(() => {
        expect(
          within(menu).getByRole("menuitem", { name: "Play" }),
        ).not.toHaveAttribute("aria-disabled", "true");
      });
    });

    it("opens the clicked instance folder", async () => {
      await renderAt("/?instance=classic");
      await openInstanceContextMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "Open folder" }));
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("open_instance_folder", {
          folder: "next",
        });
      });
    });

    it("opens configuration for the clicked instance", async () => {
      await renderAt("/?instance=classic");
      await openInstanceContextMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "Configure" }));
      expect(await screen.findByLabelText("Name")).toHaveValue("Next");
      expect(screen.getByRole("dialog")).toBeVisible();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("renames the clicked instance through the action dialog", async () => {
      await renderAt("/?instance=classic");
      await openInstanceContextMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "Rename" }));
      const dialog = await screen.findByRole("dialog", { name: "Rename Next" });
      const field = within(dialog).getByLabelText("Name");
      const save = within(dialog).getByRole("button", {
        name: "Rename instance",
      });
      expect(field).toHaveValue("Next");
      expect(save).toBeDisabled();
      fireEvent.change(field, { target: { value: "   " } });
      expect(save).toBeDisabled();
      fireEvent.change(field, { target: { value: " Renamed " } });
      fireEvent.click(save);
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("rename_instance", {
          folder: "next",
          name: "Renamed",
        });
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
      const sidebar = screen.getByRole("navigation", { name: "Instances" });
      expect(
        await within(sidebar).findByRole("button", { name: "Renamed" }),
      ).toBeVisible();
    });

    it("duplicates the clicked instance after entering a name", async () => {
      await renderAt("/?instance=classic");
      await openInstanceContextMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "Duplicate" }));
      const dialog = await screen.findByRole("dialog", {
        name: "Duplicate Next",
      });
      expect(invoke).not.toHaveBeenCalledWith(
        "duplicate_instance",
        expect.anything(),
      );
      fireEvent.change(within(dialog).getByLabelText("Name"), {
        target: { value: "Next copy" },
      });
      fireEvent.click(
        within(dialog).getByRole("button", { name: "Duplicate instance" }),
      );
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("duplicate_instance", {
          folder: "next",
          name: "Next copy",
        });
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
      expect(
        await screen.findByRole("button", { name: "Next copy" }),
      ).toBeVisible();
    });

    it("deletes only the clicked instance after confirmation", async () => {
      await renderAt("/?instance=classic");
      await openInstanceContextMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
      const dialog = await screen.findByRole("dialog", {
        name: "Delete Next?",
      });
      expect(invoke).not.toHaveBeenCalledWith(
        "delete_instance",
        expect.anything(),
      );
      fireEvent.click(
        within(dialog).getByRole("button", { name: "Delete instance" }),
      );
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("delete_instance", {
          folder: "next",
        });
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
      const sidebar = screen.getByRole("navigation", { name: "Instances" });
      await waitFor(() => {
        expect(
          within(sidebar).queryByRole("button", { name: "Next" }),
        ).not.toBeInTheDocument();
      });
      expect(
        within(sidebar).getByRole("button", { name: "Classic" }),
      ).toBeVisible();
    });
  });

  it("shows a network toast when the list fails", async () => {
    mockContextMenuCommands((command) => {
      if (command === "list_instances") {
        return Promise.reject(new Error("HTTP request failed"));
      }
    });
    await renderAt("/");
    expect(
      await screen.findByText(
        "The network request failed. Check your connection and try again.",
      ),
    ).toBeInTheDocument();
  });
});
