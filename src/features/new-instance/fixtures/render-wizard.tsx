import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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

export { invoke };

let unexpectedCommands: string[] = [];
afterEach(() => expect(unexpectedCommands).toEqual([]));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
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

export const wizardVersions: VersionSummary[] = [
  { id: "1.20.1", versionType: "release", installed: false },
  { id: "24w14a", versionType: "snapshot", installed: false },
];

export function mockWizardCommands(
  implement?: (command: string, args?: unknown) => unknown,
) {
  let instances: InstanceEntry[] = [];
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
        return wizardVersions;
      case "list_running_games":
      case "list_download_jobs":
      case "list_microsoft_accounts":
        return [];
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
      case "search_modpacks":
        return [
          {
            id: "ABC",
            source: "modrinth",
            name: "Cobblemon",
            description: "Pokemon",
            downloads: 10,
            iconUrl: null,
            author: "team",
          },
        ];
      case "list_modpack_versions":
        return [
          {
            id: "v1",
            name: "1.0.0",
            gameVersions: ["1.20.1"],
            loaders: ["fabric"],
          },
        ];
      case "create_instance_with_platform":
      case "create_instance_from_modpack": {
        if (
          args === null ||
          typeof args !== "object" ||
          !("name" in args) ||
          typeof args.name !== "string"
        ) {
          throw new Error(`Missing instance name for ${command}`);
        }
        const entry: InstanceEntry = {
          folder: args.name,
          name: args.name,
          versionId:
            "gameVersion" in args && typeof args.gameVersion === "string"
              ? args.gameVersion
              : "1.20.1",
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
        instances = [...instances, entry];
        return entry;
      }
      default:
        unexpectedCommands.push(command);
        throw new Error(`Unexpected wizard command: ${command}`);
    }
  });
}

export async function renderWizard(initialEntry = "/instances/new") {
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

export async function selectChoice(name: string | RegExp) {
  fireEvent.click(await screen.findByRole("button", { name }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name })).toHaveAttribute(
      "aria-pressed",
      "true",
    ),
  );
}

export async function continueWizard() {
  const button = screen.getByRole("button", { name: "Continue" });
  await waitFor(() => expect(button).toBeEnabled());
  fireEvent.click(button);
}
