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

describe("AppearanceSection", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("shows the color theme without developer window controls", async () => {
    await renderSettings();
    expect(await screen.findByRole("button", { name: "Dark" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(
      screen.getByRole("group", { name: "Color theme" }),
    ).toHaveAccessibleDescription("Choose the launcher's color theme.");
    expect(screen.getByRole("heading", { name: "General" })).toBeVisible();
    expect(
      screen.getByText("Accounts, appearance and launcher behavior."),
    ).toBeInTheDocument();
    expect(screen.getByText("Dark")).toBeInTheDocument();
    const appearance = within(
      screen.getByRole("region", { name: "Appearance" }),
    );
    expect(appearance.getByRole("button", { name: "Dark" })).toBeEnabled();
    expect(appearance.queryByRole("switch")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Username")).not.toBeInTheDocument();
  });

  it("offers light, dark and system cards and persists the selected theme", async () => {
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
    await renderSettings();
    const theme = await screen.findByRole("button", { name: "Dark" });
    const themes = within(screen.getByRole("group", { name: "Color theme" }));
    expect(themes.getAllByRole("button")).toHaveLength(3);
    expect(theme).toHaveAttribute("aria-pressed", "true");
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
    fireEvent.click(themes.getByRole("button", { name: "Light" }));
    expect(themes.getByRole("button", { name: "Light" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(theme).toHaveAttribute("aria-pressed", "false");
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ theme: "light" }),
      }),
    );
    fireEvent.click(themes.getByRole("button", { name: "System" }));
    expect(themes.getByRole("button", { name: "System" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ theme: "system" }),
      }),
    );
  });

  it("saves the transparency preference and offers a restart", async () => {
    mockSettingsCommands((command, args) => {
      if (command === "save_settings") {
        return Promise.resolve((args as { settings: unknown }).settings);
      }
      return undefined;
    });
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    const toggle = screen.getByRole("switch", { name: "Disable Acrylic" });
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

  it("dismisses the restart notification with Later without restarting", async () => {
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
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    fireEvent.click(screen.getByRole("switch", { name: "Disable Acrylic" }));
    expect(await screen.findByText("Restart required")).toBeVisible();
    expect(screen.getByRole("button", { name: "Restart" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Later" }));
    await waitFor(() =>
      expect(screen.queryByText("Restart required")).not.toBeInTheDocument(),
    );
    expect(invoke).not.toHaveBeenCalledWith("restart_app");
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
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    const toggle = screen.getByRole("switch", { name: "Disable Acrylic" });
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
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    const transparency = screen.getByRole("switch", {
      name: "Disable Acrylic",
    });
    const titleBar = screen.getByRole("switch", {
      name: "System title bar",
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
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    fireEvent.click(screen.getByRole("switch", { name: "System title bar" }));

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
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    const toggle = screen.getByRole("switch", {
      name: "System title bar",
    });

    fireEvent.click(toggle);
    expect(await screen.findByText("Restart required")).toBeInTheDocument();

    fireEvent.click(toggle);
    await waitFor(() => {
      expect(screen.queryByText("Restart required")).not.toBeInTheDocument();
    });
  });
});
