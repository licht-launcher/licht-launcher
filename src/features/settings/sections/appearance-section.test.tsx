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
    const hint = screen.getByText(/This change applies after restart\./);
    expect(hint).toHaveClass("text-destructive");
    expect(screen.queryByLabelText("Username")).not.toBeInTheDocument();
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
});
