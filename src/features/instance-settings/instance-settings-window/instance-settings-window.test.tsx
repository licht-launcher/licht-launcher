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
import { InstanceSettingsWindow } from "@/features/instance-settings/instance-settings-window";
import {
  invoke,
  mockSettingsCommands,
} from "@/features/settings/fixtures/render-settings";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

type CloseRequest = { preventDefault: () => void };

const {
  listeners,
  showWindow,
  setTitle,
  emitTo,
  destroyWindow,
  closeRequested,
} = vi.hoisted(() => ({
  listeners: new Map<string, Set<(event: { payload: unknown }) => void>>(),
  showWindow: vi.fn(),
  setTitle: vi.fn(),
  emitTo: vi.fn(),
  destroyWindow: vi.fn(),
  closeRequested: {
    handler: null as ((request: CloseRequest) => Promise<void>) | null,
  },
}));
vi.mock("@tauri-apps/api/event", () => ({
  emitTo,
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
    setTitle,
    destroy: destroyWindow,
    minimize: vi.fn(),
    close: vi.fn(),
    onCloseRequested: vi.fn(
      async (handler: (request: CloseRequest) => Promise<void>) => {
        closeRequested.handler = handler;
        return () => {};
      },
    ),
  }),
}));
vi.mock("@tauri-apps/api/core", async (importOriginal) => {
  const original =
    await importOriginal<typeof import("@tauri-apps/api/core")>();
  const { invoke } = await import("@/features/settings/fixtures/invoke");
  return { ...original, invoke };
});

const survival: InstanceEntry = {
  folder: "survival",
  name: "Survival",
  versionId: "1.20.1",
  minMemoryMb: 512,
  maxMemoryMb: 2048,
  jvmArguments: [],
  fullscreen: false,
  width: 854,
  height: 480,
  overrideWindow: false,
  overrideMemory: false,
  overrideJava: false,
  overrideJvmArguments: false,
  javaPath: null,
  lastOpenedAt: null,
  iconPath: null,
  customIcon: false,
};

function emit(name: string, payload: unknown) {
  act(() => {
    for (const handler of listeners.get(name) ?? []) handler({ payload });
  });
}

function view() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <InstanceSettingsWindow />
      <Toaster />
    </QueryClientProvider>,
  );
}

describe("InstanceSettingsWindow", () => {
  beforeEach(() => {
    listeners.clear();
    showWindow.mockReset().mockResolvedValue(undefined);
    setTitle.mockReset().mockResolvedValue(undefined);
    emitTo.mockReset().mockResolvedValue(undefined);
    destroyWindow.mockReset().mockResolvedValue(undefined);
    closeRequested.handler = null;
    mockSettingsCommands((command) => {
      if (command === "instance_settings_window_ready")
        return Promise.resolve("survival");
      if (command === "list_instances") return Promise.resolve([survival]);
      if (command === "save_instance") return Promise.resolve(survival);
      if (command === "get_instance_platform") {
        return Promise.resolve({
          platform: "vanilla",
          gameVersion: "1.20.1",
          loaderVersion: null,
        });
      }
      return undefined;
    });
  });
  afterEach(() => {
    toast.dismiss();
  });

  it("opens the instance it was created for and then shows itself", async () => {
    view();
    expect(
      await screen.findByRole("heading", { level: 1, name: "Survival" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "General", current: "page" }),
    ).toBeVisible();
    expect(await screen.findByLabelText("Name")).toHaveValue("Survival");
    await waitFor(() => expect(showWindow).toHaveBeenCalledOnce());
    expect(setTitle).toHaveBeenCalledWith("Survival - Licht Launcher");
    expect(emitTo).toHaveBeenCalledWith(
      "main",
      "instance-settings-status-request",
    );
  });

  it("shows itself even when it cannot ask for its instance", async () => {
    mockSettingsCommands((command) =>
      command === "instance_settings_window_ready"
        ? Promise.reject(new Error("No instance"))
        : undefined,
    );
    view();
    await waitFor(() => expect(showWindow).toHaveBeenCalledOnce());
    expect(await screen.findByText("No instance")).toBeVisible();
  });

  it("asks the launcher window to run instance actions", async () => {
    view();
    const name = await screen.findByLabelText("Name");
    fireEvent.change(name, { target: { value: "Hardcore" } });
    fireEvent.blur(name);
    fireEvent.click(screen.getByRole("button", { name: "Duplicate" }));
    expect(emitTo).toHaveBeenCalledWith("main", "instance-settings-action", {
      kind: "rename",
      folder: "survival",
      name: "Hardcore",
    });
    expect(emitTo).toHaveBeenCalledWith("main", "instance-settings-action", {
      kind: "duplicate",
      folder: "survival",
    });
  });

  it("asks before deleting the instance", async () => {
    view();
    fireEvent.click(
      await screen.findByRole("button", { name: "Delete instance" }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Delete Survival?",
    });
    expect(emitTo).not.toHaveBeenCalledWith(
      "main",
      "instance-settings-action",
      expect.anything(),
    );
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Delete instance" }),
    );
    expect(emitTo).toHaveBeenCalledWith("main", "instance-settings-action", {
      kind: "delete",
      folder: "survival",
    });
  });

  it("holds the actions while the launcher is busy", async () => {
    view();
    const duplicate = await screen.findByRole("button", { name: "Duplicate" });
    expect(duplicate).toBeEnabled();
    emit("instance-settings-status", {
      busy: true,
      versionBusy: false,
      activeFolders: [],
    });
    expect(duplicate).toBeDisabled();
  });

  it("shows why an action failed only for its own instance", async () => {
    view();
    await screen.findByLabelText("Name");
    emit("instance-settings-action-failed", {
      folder: "creative",
      message: "Not this one",
    });
    emit("instance-settings-action-failed", {
      folder: "survival",
      message: "Folder in use",
    });
    expect(await screen.findByText("Folder in use")).toBeVisible();
    expect(screen.queryByText("Not this one")).not.toBeInTheDocument();
  });

  it("refetches instances changed by another window", async () => {
    view();
    await screen.findByLabelText("Name");
    const fetches = () =>
      invoke.mock.calls.filter(([command]) => command === "list_instances")
        .length;
    const before = fetches();
    await waitFor(() => expect(listeners.has("instances-changed")).toBe(true));
    emit("instances-changed", null);
    await waitFor(() => expect(fetches()).toBeGreaterThan(before));
  });

  it("saves the last edit before the window closes", async () => {
    view();
    fireEvent.click(
      await screen.findByRole("button", { name: "Override Settings" }),
    );
    fireEvent.click(
      screen.getByRole("switch", { name: "Customize window settings" }),
    );
    expect(invoke).not.toHaveBeenCalledWith("save_instance", expect.anything());
    const request = { preventDefault: vi.fn() };
    await act(async () => {
      await closeRequested.handler?.(request);
    });
    expect(invoke).toHaveBeenCalledWith("save_instance", {
      folder: "survival",
      settings: expect.objectContaining({ overrideWindow: true }),
    });
    expect(request.preventDefault).not.toHaveBeenCalled();
  });

  it("stays open when the last edit cannot be saved", async () => {
    view();
    fireEvent.click(
      await screen.findByRole("button", { name: "Override Settings" }),
    );
    fireEvent.click(
      screen.getByRole("switch", { name: "Customize window settings" }),
    );
    fireEvent.change(screen.getByLabelText("Width"), {
      target: { value: "0" },
    });
    const request = { preventDefault: vi.fn() };
    await act(async () => {
      await closeRequested.handler?.(request);
    });
    expect(request.preventDefault).toHaveBeenCalled();
    expect(
      await screen.findByText("window size must be at least 1"),
    ).toBeVisible();
  });

  it("saves the last edit and closes when the launcher closes", async () => {
    view();
    fireEvent.click(
      await screen.findByRole("button", { name: "Override Settings" }),
    );
    fireEvent.click(
      screen.getByRole("switch", { name: "Customize window settings" }),
    );
    await waitFor(() => expect(listeners.has("launcher-closed")).toBe(true));
    await act(async () => {
      for (const handler of listeners.get("launcher-closed") ?? [])
        handler({ payload: null });
    });
    await waitFor(() => expect(destroyWindow).toHaveBeenCalled());
    expect(invoke).toHaveBeenCalledWith("save_instance", {
      folder: "survival",
      settings: expect.objectContaining({ overrideWindow: true }),
    });
  });
});
