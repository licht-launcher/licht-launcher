import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

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

describe("SettingsSidebar", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("shows the app name, version, and opens the repository", async () => {
    await renderSettings();
    await screen.findByLabelText("Offline username");

    expect(screen.getByRole("heading", { name: "Settings" })).toBeVisible();
    expect(screen.getByText("Licht Launcher")).toBeInTheDocument();
    expect(screen.getByText("0.2.2 · Iris")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "GitHub repository" }));
    expect(invoke).toHaveBeenCalledWith("open_repository");
  });
});
