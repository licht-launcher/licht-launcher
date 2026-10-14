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
    expect(
      await screen.findByLabelText("Offline username"),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Save" }),
    ).not.toBeInTheDocument();
  });

  it("does not save just by opening", async () => {
    await renderSettings();
    await screen.findByLabelText("Offline username");
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });

  it("keeps the application titlebar interactive while open", async () => {
    await renderSettings();
    await screen.findByLabelText("Offline username");

    expect(document.body).not.toHaveStyle({ pointerEvents: "none" });
  });

  it("keeps the modal backdrop visible while open", async () => {
    await renderSettings();
    await screen.findByLabelText("Offline username");

    expect(document.querySelector(".bg-black\\/60")).toBeInTheDocument();
  });
});
