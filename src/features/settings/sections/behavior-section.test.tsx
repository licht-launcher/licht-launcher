import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  invoke,
  mockSettingsCommands,
  renderSettings,
  snapshot,
} from "@/features/settings/fixtures/render-settings";

vi.mock("@tauri-apps/api/core", async () => {
  const { invoke: mocked } = await import(
    "@/features/settings/fixtures/invoke"
  );
  return { invoke: mocked };
});

describe("General preferences", () => {
  beforeEach(() => {
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
  });

  it("offers English as the only language and enables minimize-to-tray by default", async () => {
    await renderSettings();
    const language = await screen.findByRole("combobox", { name: "Language" });
    expect(language).toHaveTextContent("English");
    expect(
      screen.getByRole("switch", {
        name: "Use hardware acceleration when available",
      }),
    ).toBeChecked();
    expect(
      screen.getByRole("switch", { name: "Minimize to tray on close" }),
    ).toBeChecked();
    expect(
      screen.getByRole("switch", {
        name: "Minimize launcher when Minecraft starts",
      }),
    ).not.toBeChecked();
    expect(
      screen.queryByRole("switch", {
        name: "Restore launcher when Minecraft closes",
      }),
    ).not.toBeInTheDocument();
    fireEvent.keyDown(language, { key: "ArrowDown" });
    const options = within(await screen.findByRole("listbox")).getAllByRole(
      "option",
    );
    expect(options).toHaveLength(1);
    expect(options[0]).toHaveAccessibleName("English");
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });

  it("persists hardware acceleration and dismisses the restart prompt when reverted", async () => {
    await renderSettings();
    const acceleration = await screen.findByRole("switch", {
      name: "Use hardware acceleration when available",
    });
    fireEvent.click(acceleration);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ hardwareAcceleration: false }),
      }),
    );
    expect(
      await screen.findByText("Apply the hardware acceleration change now?"),
    ).toBeVisible();
    fireEvent.click(acceleration);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ hardwareAcceleration: true }),
      }),
    );
    await waitFor(() =>
      expect(screen.queryByText("Restart required")).not.toBeInTheDocument(),
    );
  });

  it("keeps a restart pending while another restart-dependent preference is changed", async () => {
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("switch", {
        name: "Use hardware acceleration when available",
      }),
    );
    expect(await screen.findByText("Restart required")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Developer Tools" }));
    fireEvent.click(screen.getByRole("switch", { name: "System title bar" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ disableCustomTitleBar: true }),
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "General" }));
    fireEvent.click(
      await screen.findByRole("switch", {
        name: "Use hardware acceleration when available",
      }),
    );
    expect(screen.getByText("Restart required")).toBeVisible();
    await waitFor(() =>
      expect(screen.getByText("Apply the title bar change now?")).toBeVisible(),
    );
  });

  it("saves minimize-to-tray without requesting a restart", async () => {
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("switch", { name: "Minimize to tray on close" }),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ minimizeToTray: false }),
      }),
    );
    expect(screen.queryByText("Restart required")).not.toBeInTheDocument();
  });

  it("shows restore only while minimizing on game start is enabled and saves both choices", async () => {
    await renderSettings();
    const minimize = await screen.findByRole("switch", {
      name: "Minimize launcher when Minecraft starts",
    });
    fireEvent.click(minimize);
    const restore = await screen.findByRole("switch", {
      name: "Restore launcher when Minecraft closes",
    });
    expect(restore).not.toBeChecked();
    fireEvent.click(restore);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({
          minimizeOnGameStart: true,
          restoreOnGameExit: true,
        }),
      }),
    );
    fireEvent.click(minimize);
    expect(
      screen.queryByRole("switch", {
        name: "Restore launcher when Minecraft closes",
      }),
    ).not.toBeInTheDocument();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ minimizeOnGameStart: false }),
      }),
    );
    fireEvent.click(minimize);
    expect(
      await screen.findByRole("switch", {
        name: "Restore launcher when Minecraft closes",
      }),
    ).toBeChecked();
  });

  it("loads persisted behavior preferences", async () => {
    mockSettingsCommands((command) =>
      command === "get_settings"
        ? Promise.resolve({
            ...snapshot,
            settings: {
              ...snapshot.settings,
              hardwareAcceleration: false,
              minimizeToTray: false,
              minimizeOnGameStart: true,
              restoreOnGameExit: true,
            },
          })
        : undefined,
    );
    await renderSettings();
    expect(
      await screen.findByRole("switch", {
        name: "Use hardware acceleration when available",
      }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("switch", { name: "Minimize to tray on close" }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("switch", {
        name: "Minimize launcher when Minecraft starts",
      }),
    ).toBeChecked();
    expect(
      screen.getByRole("switch", {
        name: "Restore launcher when Minecraft closes",
      }),
    ).toBeChecked();
    expect(screen.queryByText("Restart required")).not.toBeInTheDocument();
  });
});
