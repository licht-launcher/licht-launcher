import { screen } from "@testing-library/react";
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

describe("SettingsDialog", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("does not show a Save button", async () => {
    await renderSettings();
    expect(await screen.findByLabelText("Username")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Save" }),
    ).not.toBeInTheDocument();
  });

  it("does not save just by opening", async () => {
    await renderSettings();
    await screen.findByLabelText("Username");
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });
});
