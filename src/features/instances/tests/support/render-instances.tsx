import "@/features/instances/tests/support/tauri-mocks";

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
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, vi } from "vitest";

import { Toaster } from "@/components/ui/sonner";
import { completedDownload } from "@/features/downloads/fixtures/job";
import { storageSummary } from "@/features/downloads/fixtures/storage";
import {
  closeWindow,
  invoke,
  listeners,
} from "@/features/instances/tests/support/tauri-mocks";
import {
  playerHeadPathFixture,
  snapshot,
} from "@/features/settings/fixtures/snapshot";
import { startupProgress } from "@/features/startup/fixtures/progress";
import { routeTree } from "@/routeTree.gen";

export { closeWindow, invoke, listeners };

export async function renderAt(path: string) {
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

export const classicInstance = {
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

export const contextInstance = {
  ...classicInstance,
  folder: "next",
  name: "Next",
  versionId: "1.20.1",
};

export function mockContextMenuCommands(
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
    if (command === "runtime_java") {
      return Promise.resolve({
        java25: null,
        java21: null,
        java17: null,
        java8: null,
      });
    }
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

export async function openInstanceContextMenu(name = "Next") {
  const sidebar = await screen.findByRole("navigation", { name: "Instances" });
  const target = await within(sidebar).findByRole("button", {
    name: new RegExp(`^${name}(?: \\(|$)`),
  });
  fireEvent.contextMenu(target, { clientX: 50, clientY: 60 });
  return screen.findByRole("menu", { name: `Actions for ${name}` });
}

export function setupInstancesPageTests() {
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
}
