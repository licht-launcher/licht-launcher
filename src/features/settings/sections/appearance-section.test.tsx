import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  mockSettingsCommands,
  renderSettings,
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
    expect(screen.queryByLabelText("Username")).not.toBeInTheDocument();
  });
});
