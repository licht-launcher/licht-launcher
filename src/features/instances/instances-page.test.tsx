import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Toaster } from "@/components/ui/sonner";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import type { GameLogEvent } from "@/lib/generated/GameLogEvent";
import type { InstallProgress } from "@/lib/generated/InstallProgress";
import { routeTree } from "@/routeTree.gen";

const { invoke, listeners } = vi.hoisted(() => ({
  invoke: vi.fn(),
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
    close: vi.fn(),
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
  const view = render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <Toaster />
    </QueryClientProvider>,
  );
  return { ...view, router };
}

function defaultCommandResponse(command: string): Promise<unknown> {
  switch (command) {
    case "custom_title_bar_active":
      return Promise.resolve(true);
    case "list_instance_content":
      return Promise.resolve([]);
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

describe("InstancesPage", () => {
  beforeEach(() => {
    listeners.clear();
    invoke.mockReset();
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
    fireEvent.click(await screen.findByRole("option", { name: "Name" }));
    expect(names()).toEqual(["Alpha", "Classic", "Zeta"]);

    fireEvent.click(screen.getByRole("combobox", { name: "Sort instances" }));
    fireEvent.click(await screen.findByRole("option", { name: "Last played" }));
    expect(names()).toEqual(["Classic", "Zeta", "Alpha"]);
  });

  it("shows the sidebar empty state and opens settings", async () => {
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

    expect(await screen.findByText("No instances yet.")).toBeInTheDocument();
    expect(
      screen.getByText("Create an instance to get started."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "New instance" })).toBeEnabled();
    expect(screen.getByText("Licht Launcher")).toBeInTheDocument();
    expect(
      screen.queryByRole("list", { name: "Versions" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("combobox", { name: "Type" }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(
      await screen.findByRole("button", {
        name: "Account",
        current: "page",
      }),
    ).toBeInTheDocument();
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
    expect(screen.getByRole("button", { name: "New instance" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Settings" })).toBeEnabled();
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
      "install_version",
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
    let finishInstall: (java: string) => void = () => {};
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
      if (command === "install_version") {
        return new Promise<string>((resolve) => {
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
      expect(invoke).toHaveBeenCalledWith("install_version", {
        versionId: "1.20.1",
      });
    });
    expect(
      await screen.findByRole("progressbar", { name: "Install progress" }),
    ).toBeInTheDocument();

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

    finishInstall("java");
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("install_modpack_files", {
        folder: "next",
      });
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
      "install_version",
      expect.anything(),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "install_modpack_files",
      expect.anything(),
    );
  });

  it("keeps Preparing while install progress has no totals yet", async () => {
    let finishInstall: (java: string) => void = () => {};
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
      if (command === "install_version") {
        return new Promise<string>((resolve) => {
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

    finishInstall("java");
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "modded",
        username: "Steve",
      });
    });
  });

  it("installs two versions in parallel without locking the sidebar", async () => {
    const finishInstall: Record<string, (java: string) => void> = {};
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
      if (command === "install_version") {
        const versionId = (args as { versionId: string }).versionId;
        return new Promise<string>((resolve) => {
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
      expect(invoke).toHaveBeenCalledWith("install_version", {
        versionId: "1.20.1",
      });
    });
    expect(screen.getByRole("button", { name: "New instance" })).toBeEnabled();
    expect(
      within(sidebar).getByRole("button", { name: /^Fresh/ }),
    ).toBeEnabled();

    fireEvent.click(within(sidebar).getByRole("button", { name: /^Fresh/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("install_version", {
        versionId: "1.21.1",
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

    finishInstall["1.20.1"]?.("java");
    finishInstall["1.21.1"]?.("java");
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

  it("opens the new instance wizard from the sidebar", async () => {
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
    fireEvent.click(
      await screen.findByRole("button", { name: "New instance" }),
    );
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeInTheDocument();
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

    fireEvent.click(screen.getByRole("button", { name: "New instance" }));
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Forward" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(
      await screen.findByText("Create an instance to get started."),
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

  it("shows a network toast when the list fails", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error("HTTP request failed"));
    });
    await renderAt("/");
    expect(
      await screen.findByText(
        "The network request failed. Check your connection and try again.",
      ),
    ).toBeInTheDocument();
  });
});
