import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CloseRequest } from "@/api/window-close";
import {
  invoke,
  mockSettingsCommands,
  renderSettings,
} from "@/features/settings/fixtures/render-settings";

vi.mock("@tauri-apps/api/core", async () => {
  const { invoke: mocked } = await import(
    "@/features/settings/fixtures/invoke"
  );
  return { invoke: mocked };
});

const { closeHandlers, launcherClosedHandlers } = vi.hoisted(() => ({
  closeHandlers: new Set<(request: CloseRequest) => Promise<void>>(),
  launcherClosedHandlers: new Set<() => Promise<void>>(),
}));
vi.mock("@/api/window-close", () => ({
  onWindowCloseRequested: async (
    handler: (request: CloseRequest) => Promise<void>,
  ) => {
    closeHandlers.add(handler);
    return () => {
      closeHandlers.delete(handler);
    };
  },
  onLauncherClosed: async (handler: () => Promise<void>) => {
    launcherClosedHandlers.add(handler);
    return () => {
      launcherClosedHandlers.delete(handler);
    };
  },
}));

/** Asks this window to close, as its close button does, and waits for the page. */
async function requestClose() {
  const request = { preventDefault: vi.fn() };
  await waitFor(() => expect(closeHandlers.size).toBe(1));
  for (const handler of closeHandlers) await handler(request);
  return request;
}

describe("SettingsPage", () => {
  beforeEach(() => {
    closeHandlers.clear();
    launcherClosedHandlers.clear();
    mockSettingsCommands();
  });

  it("groups accounts and appearance under General in order", async () => {
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });

    expect(screen.getByRole("heading", { name: "General" })).toBeVisible();
    expect(
      screen
        .getAllByRole("heading", { level: 3 })
        .map((heading) => heading.textContent),
    ).toEqual(["Account", "Appearance", "Behavior"]);
    expect(screen.getByRole("button", { name: "Dark" })).toHaveTextContent(
      "Dark",
    );
  });

  it("does not show a Save button", async () => {
    await renderSettings();
    expect(
      await screen.findByRole("button", { name: "Dark" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Save" }),
    ).not.toBeInTheDocument();
  });

  it("preserves edited fields and active input focus across section transitions", async () => {
    mockSettingsCommands((command, args) => {
      if (
        command === "save_settings" &&
        args !== null &&
        typeof args === "object" &&
        "settings" in args
      ) {
        return Promise.resolve(args.settings);
      }
      return undefined;
    });
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    fireEvent.change(await screen.findByLabelText("Offline username"), {
      target: { value: "Builder" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Downloads" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Limit download speed" }),
    );
    const limit = await screen.findByLabelText("Speed limit (kB/s)");
    limit.focus();
    fireEvent.change(limit, { target: { value: "8" } });
    expect(screen.getByLabelText("Speed limit (kB/s)")).toBe(limit);
    expect(limit).toHaveFocus();
    expect(limit).toHaveValue(8);

    fireEvent.click(screen.getByRole("button", { name: "Developer Tools" }));
    expect(await screen.findByLabelText("Offline username")).toHaveValue(
      "Builder",
    );
    fireEvent.click(screen.getByRole("button", { name: "Downloads" }));
    expect(await screen.findByLabelText("Speed limit (kB/s)")).toHaveValue(8);
  });

  it("shows the offline username directly in Developer Tools and saves edits", async () => {
    mockSettingsCommands((command, args) => {
      if (
        command === "save_settings" &&
        typeof args === "object" &&
        args !== null &&
        "settings" in args
      ) {
        return Promise.resolve(args.settings);
      }
      return undefined;
    });
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });
    expect(
      screen.queryByRole("button", { name: "Offline username" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Developer Tools" }));
    expect(
      await screen.findByRole("heading", { name: "Developer Tools" }),
    ).toBeVisible();
    fireEvent.change(
      screen.getByRole("textbox", { name: "Offline username" }),
      { target: { value: "Builder" } },
    );
    expect(
      screen.getByRole("textbox", { name: "Offline username" }),
    ).toHaveValue("Builder");
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ username: "Builder" }),
      });
    });
  });

  it("does not save just by opening", async () => {
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });

  it("saves a pending edit when leaving the page", async () => {
    const view = await renderSettings("developers");
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Offline username" }),
      { target: { value: "Leaving" } },
    );
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());

    view.unmount();

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ username: "Leaving" }),
      }),
    );
  });

  it("saves a pending edit before the window closes", async () => {
    mockSettingsCommands((command, args) => {
      if (
        command === "save_settings" &&
        typeof args === "object" &&
        args !== null &&
        "settings" in args
      )
        return Promise.resolve(args.settings);
      return undefined;
    });
    await renderSettings("developers");
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Offline username" }),
      { target: { value: "Closing" } },
    );

    const request = await requestClose();

    expect(invoke).toHaveBeenCalledWith("save_settings", {
      settings: expect.objectContaining({ username: "Closing" }),
    });
    expect(request.preventDefault).not.toHaveBeenCalled();
  });

  it("keeps the window open when the last edit cannot be saved", async () => {
    mockSettingsCommands((command) =>
      command === "save_settings"
        ? Promise.reject(new Error("Disk is full"))
        : undefined,
    );
    await renderSettings("developers");
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Offline username" }),
      { target: { value: "Closing" } },
    );

    const request = await requestClose();

    expect(request.preventDefault).toHaveBeenCalledOnce();
    expect(await screen.findByText("Disk is full")).toBeVisible();
  });

  it("closes right away when nothing is waiting to be saved", async () => {
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });

    const request = await requestClose();

    expect(request.preventDefault).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });

  it("waits for a save already running before the window closes", async () => {
    let failSave: ((error: Error) => void) | undefined;
    mockSettingsCommands((command) =>
      command === "save_settings"
        ? new Promise((_resolve, reject) => {
            failSave = reject;
          })
        : undefined,
    );
    await renderSettings("developers");
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Offline username" }),
      { target: { value: "Saving" } },
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", expect.anything()),
    );

    const closing = requestClose();
    await new Promise((resolve) => setTimeout(resolve, 20));
    failSave?.(new Error("Disk is full"));
    const request = await closing;

    expect(request.preventDefault).toHaveBeenCalledOnce();
    expect(await screen.findByText("Disk is full")).toBeVisible();
  });

  it("saves the last edit when the launcher closes", async () => {
    mockSettingsCommands((command, args) => {
      if (
        command === "save_settings" &&
        typeof args === "object" &&
        args !== null &&
        "settings" in args
      )
        return Promise.resolve(args.settings);
      return undefined;
    });
    await renderSettings("developers");
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Offline username" }),
      { target: { value: "Leaving" } },
    );
    await waitFor(() => expect(launcherClosedHandlers.size).toBe(1));

    for (const handler of launcherClosedHandlers) await handler();

    expect(invoke).toHaveBeenCalledWith("save_settings", {
      settings: expect.objectContaining({ username: "Leaving" }),
    });
  });

  it("reports an edit it could not save when the launcher closes", async () => {
    await renderSettings("instances");
    fireEvent.change(await screen.findByLabelText("Window width"), {
      target: { value: "0" },
    });
    await waitFor(() => expect(launcherClosedHandlers.size).toBe(1));

    for (const handler of launcherClosedHandlers) await handler();

    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
    expect(invoke).toHaveBeenCalledWith("log_frontend_error", {
      source: "promise",
      message: expect.stringContaining(
        "could not be saved before the launcher closed",
      ),
    });
  });
});
