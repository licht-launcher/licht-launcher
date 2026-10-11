import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
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
import { beforeEach, describe, expect, it, vi } from "vitest";

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
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <Toaster />
    </QueryClientProvider>,
  );
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
};

describe("InstancesPage", () => {
  beforeEach(() => {
    listeners.clear();
    invoke.mockReset();
  });

  it("shows the sidebar empty state and opens settings", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
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
      return Promise.resolve(undefined);
    });

    await renderAt("/");

    expect(await screen.findByText("No instances yet.")).toBeInTheDocument();
    expect(
      screen.getByText("Create an instance to get started."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "New instance" })).toBeEnabled();
    expect(screen.getAllByText("0.1.0 · Lilie").length).toBeGreaterThan(0);
    expect(
      screen.getByText(
        "Unofficial, not affiliated with Mojang Studios or Microsoft.",
      ),
    ).toBeInTheDocument();
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
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
      if (command === "list_instances") {
        return Promise.resolve([classicInstance]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.5.2", versionType: "release", installed: true },
          { id: "24w14a", versionType: "snapshot", installed: false },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve({
          ...snapshot,
          settings: { ...snapshot.settings, username: "Mateus" },
        });
      }
      if (command === "launch") {
        return new Promise<{ code: number }>((resolve) => {
          finishLaunch = resolve;
        });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "stop_game") {
        return Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });

    await renderAt("/");

    const sidebar = await screen.findByRole("navigation", {
      name: "Instances",
    });
    expect(within(sidebar).getByText("Classic")).toBeInTheDocument();
    expect(screen.queryByLabelText("Username")).not.toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Classic" }),
    ).toBeInTheDocument();
    expect(screen.getByText("1.5.2")).toBeInTheDocument();
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
    expect(await screen.findByText("Sound engine started")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Starting…" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();
    expect(screen.getByText("Console")).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith(
      "install_version",
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
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
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
      return Promise.resolve(undefined);
    });

    await renderAt("/");
    expect(await screen.findByRole("button", { name: "Stop" })).toBeEnabled();
    expect(
      screen.queryByRole("button", { name: "Play" }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("stop_game", { folder: "classic" });
    });
    expect(await screen.findByRole("button", { name: "Play" })).toBeEnabled();
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
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
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
      if (command === "launch" && args?.folder) {
        return new Promise<{ code: number }>((resolve) => {
          finishLaunch[args.folder as string] = resolve;
        });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
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

    fireEvent.click(within(sidebar).getByRole("button", { name: "Next" }));
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

    expect(await screen.findByText("next-log")).toBeInTheDocument();
    expect(screen.queryByText("classic-log")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();

    fireEvent.click(within(sidebar).getByRole("button", { name: "Classic" }));
    expect(await screen.findByText("classic-log")).toBeInTheDocument();
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
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
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
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "install_version") {
        return new Promise<string>((resolve) => {
          finishInstall = resolve;
        });
      }
      if (command === "launch") {
        return Promise.resolve({ code: 0 });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
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
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "next",
        username: "Steve",
      });
    });
  });

  it("creates an instance from a release that is not installed", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: false },
          { id: "24w14a", versionType: "snapshot", installed: false },
        ]);
      }
      if (command === "create_instance") {
        return Promise.resolve({
          ...classicInstance,
          folder: "Survival",
          name: "Survival",
          versionId: "1.20.1",
        });
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
    });

    await renderAt("/");
    fireEvent.click(
      await screen.findByRole("button", { name: "New instance" }),
    );
    fireEvent.change(await screen.findByLabelText("Name"), {
      target: { value: "Survival" },
    });

    fireEvent.click(screen.getByRole("combobox", { name: "Version" }));
    const releaseOption = await screen.findByRole("option", {
      name: "1.20.1 (will install)",
    });
    expect(
      screen.queryByRole("option", { name: /24w14a/ }),
    ).not.toBeInTheDocument();
    fireEvent.click(releaseOption);

    fireEvent.click(screen.getByRole("button", { name: "Create instance" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("create_instance", {
        name: "Survival",
        versionId: "1.20.1",
      });
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "install_version",
      expect.anything(),
    );
  });

  it("includes snapshots in the create dialog when the switch is on", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: true },
          { id: "24w14a", versionType: "snapshot", installed: false },
        ]);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
    });

    await renderAt("/");
    fireEvent.click(
      await screen.findByRole("button", { name: "New instance" }),
    );
    fireEvent.click(screen.getByRole("combobox", { name: "Version" }));
    expect(
      await screen.findByRole("option", { name: "1.20.1" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: /24w14a/ }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("option", { name: "1.20.1" }));

    fireEvent.click(screen.getByRole("switch", { name: "Include snapshots" }));
    fireEvent.click(screen.getByRole("combobox", { name: "Version" }));
    expect(
      await screen.findByRole("option", { name: "24w14a (will install)" }),
    ).toBeInTheDocument();
  });

  it("shows a network toast when the list fails", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_instance_entries") {
        return Promise.resolve([]);
      }
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
