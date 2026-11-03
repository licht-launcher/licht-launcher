import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, expect, vi } from "vitest";

import { Toaster } from "@/components/ui/sonner";
import { completedDownload } from "@/features/downloads/fixtures/job";
import {
  playerHeadPathFixture,
  snapshot,
} from "@/features/settings/fixtures/snapshot";
import { startupProgress } from "@/features/startup/fixtures/progress";
import { appUpdateState } from "@/features/updater/fixtures/state";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import type { VersionSummary } from "@/lib/generated/VersionSummary";
import { routeTree } from "@/routeTree.gen";

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>(),
}));

const { openDialog } = vi.hoisted(() => ({
  openDialog: vi.fn<() => Promise<string | null>>(),
}));

export { invoke, openDialog };

let unexpectedCommands: string[] = [];
afterEach(() => expect(unexpectedCommands).toEqual([]));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openDialog }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    close: vi.fn(),
    setDecorations: vi.fn(),
  }),
}));

export const createVersions: VersionSummary[] = [
  { id: "1.20.1", versionType: "release", installed: false },
  { id: "1.19.4", versionType: "release", installed: false },
  { id: "24w14a", versionType: "snapshot", installed: false },
];

/** Gives the virtualized version list a size, which jsdom does not lay out. */
export function mockVersionViewport() {
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
    function (this: HTMLElement) {
      return this.dataset.slot === "combobox-viewport" ? 256 : 0;
    },
  );
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(400);
}

export function createdInstance(name: string, versionId = "1.20.1") {
  const entry: InstanceEntry = {
    folder: name,
    name,
    versionId,
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
    lastOpenedAt: null,
    iconPath: null,
  };
  return entry;
}

export function mockCreateCommands(
  implement?: (command: string, args?: unknown) => unknown,
  existing: InstanceEntry[] = [],
) {
  let instances: InstanceEntry[] = [...existing];
  unexpectedCommands = [];
  invoke.mockReset();
  invoke.mockImplementation(async (command, args) => {
    const custom = implement?.(command, args);
    if (custom !== undefined) return custom;
    switch (command) {
      case "get_app_update_state":
        return appUpdateState();
      case "get_startup_progress":
        return startupProgress();
      case "get_player_head":
        return playerHeadPathFixture;
      case "get_settings":
        return snapshot;
      case "list_instances":
        return instances;
      case "list_versions":
        return createVersions;
      case "list_running_games":
      case "list_download_jobs":
      case "list_microsoft_accounts":
      case "list_instance_content":
        return [];
      case "get_last_played_folder":
      case "active_microsoft_account":
        return null;
      case "custom_title_bar_active":
        return true;
      case "download_telemetry":
        return { receivedBytes: 0, writtenBytes: 0, paused: false };
      case "prepare_instance_download":
        return completedDownload();
      case "list_loader_versions":
        return ["0.16.0", "0.15.11"];
      case "create_instance_with_platform":
      case "create_instance_from_modpack_file": {
        if (
          args === null ||
          typeof args !== "object" ||
          !("name" in args) ||
          typeof args.name !== "string"
        ) {
          throw new Error(`Missing instance name for ${command}`);
        }
        const entry = createdInstance(
          args.name,
          "gameVersion" in args && typeof args.gameVersion === "string"
            ? args.gameVersion
            : "1.20.1",
        );
        instances = [...instances, entry];
        return entry;
      }
      default:
        unexpectedCommands.push(command);
        throw new Error(`Unexpected command: ${command}`);
    }
  });
}

export async function renderApp(initialEntry = "/") {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [initialEntry] }),
  });
  await router.load();
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <Toaster />
    </QueryClientProvider>,
  );
}

/** Opens the dialog from the library footer. */
export async function openCreateDialog() {
  const footer = await screen.findByRole("contentinfo");
  fireEvent.click(
    within(footer).getByRole("button", { name: "Create instance" }),
  );
  return screen.findByRole("dialog", { name: "Create instance" });
}
