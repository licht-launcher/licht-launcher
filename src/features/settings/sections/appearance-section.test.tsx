import { fireEvent, screen, waitFor } from "@testing-library/react";
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

describe("AppearanceSection", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("shows the dark theme and title bar option", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Appearance" }));

    expect(await screen.findByLabelText("Color theme")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Appearance" })).toBeVisible();
    expect(screen.getByText("How the launcher looks.")).toBeInTheDocument();
    expect(screen.getByText("Dark")).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "Disable custom title bar" }),
    ).not.toBeChecked();
    expect(
      screen.getAllByText("This change applies after restart.")[0],
    ).toHaveClass("text-destructive");
    expect(screen.queryByLabelText("Username")).not.toBeInTheDocument();
  });

  it("saves the transparency preference and offers a restart", async () => {
    mockSettingsCommands((command, args) => {
      if (command === "save_settings") {
        return Promise.resolve((args as { settings: unknown }).settings);
      }
      return undefined;
    });
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Appearance" }));
    const toggle = screen.getByRole("switch", { name: "Disable transparency" });
    expect(toggle).not.toBeChecked();
    fireEvent.click(toggle);
    expect(toggle).toBeChecked();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ disableTransparency: true }),
      });
    });
    expect(
      await screen.findByText("Apply the appearance changes now?"),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Restart" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("restart_app"));
  });

  it("loads a disabled transparency preference and allows enabling it", async () => {
    mockSettingsCommands((command, args) => {
      if (command === "get_settings") {
        return Promise.resolve({
          ...snapshot,
          settings: { ...snapshot.settings, disableTransparency: true },
        });
      }
      if (command === "save_settings") {
        return Promise.resolve((args as { settings: unknown }).settings);
      }
      return undefined;
    });
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Appearance" }));
    const toggle = screen.getByRole("switch", { name: "Disable transparency" });
    expect(toggle).toBeChecked();
    fireEvent.click(toggle);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ disableTransparency: false }),
      });
    });
    expect(
      await screen.findByText("Apply the appearance changes now?"),
    ).toBeVisible();
  });

  it("keeps the restart prompt until both appearance changes are reverted", async () => {
    mockSettingsCommands((command, args) => {
      if (command === "save_settings") {
        return Promise.resolve((args as { settings: unknown }).settings);
      }
      return undefined;
    });
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Appearance" }));
    const transparency = screen.getByRole("switch", {
      name: "Disable transparency",
    });
    const titleBar = screen.getByRole("switch", {
      name: "Disable custom title bar",
    });
    fireEvent.click(transparency);
    expect(await screen.findByText("Restart required")).toBeVisible();
    fireEvent.click(titleBar);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({
          disableTransparency: true,
          disableCustomTitleBar: true,
        }),
      }),
    );
    fireEvent.click(titleBar);
    expect(screen.getByText("Restart required")).toBeVisible();
    fireEvent.click(transparency);
    await waitFor(() =>
      expect(screen.queryByText("Restart required")).not.toBeInTheDocument(),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({
          disableTransparency: false,
          disableCustomTitleBar: false,
        }),
      }),
    );
  });

  it("asks to restart after the title bar setting is saved", async () => {
    mockSettingsCommands((command) => {
      if (command === "save_settings") {
        return Promise.resolve({
          ...snapshot.settings,
          disableCustomTitleBar: true,
        });
      }
      return undefined;
    });

    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Appearance" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Disable custom title bar" }),
    );

    expect(await screen.findByText("Restart required")).toBeInTheDocument();
    expect(
      screen.getByText("Apply the title bar change now?"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Restart" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("restart_app");
    });
  });

  it("dismisses the restart toast when the title bar option is reverted", async () => {
    let savedDisableCustomTitleBar = false;
    mockSettingsCommands((command, args) => {
      if (command === "save_settings") {
        const settings = (
          args as { settings?: { disableCustomTitleBar?: boolean } } | undefined
        )?.settings;
        savedDisableCustomTitleBar = settings?.disableCustomTitleBar ?? false;
        return Promise.resolve({
          ...snapshot.settings,
          disableCustomTitleBar: savedDisableCustomTitleBar,
        });
      }
      return undefined;
    });

    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Appearance" }));
    const toggle = screen.getByRole("switch", {
      name: "Disable custom title bar",
    });

    fireEvent.click(toggle);
    expect(await screen.findByText("Restart required")).toBeInTheDocument();

    fireEvent.click(toggle);
    await waitFor(() => {
      expect(screen.queryByText("Restart required")).not.toBeInTheDocument();
    });
  });
});
