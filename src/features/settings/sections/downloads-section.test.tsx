import { fireEvent, screen, waitFor } from "@testing-library/react";
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

describe("DownloadsSection", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("saves the application directory and explains the move after restart", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));

    expect(
      screen.getByText(/Existing data will be moved after restart\./),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /Launcher settings and the database stay in your profile directory\./,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        name: "Maximum simultaneous downloads",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Maximum simultaneous writes" }),
    ).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Application directory"), {
      target: { value: "D:\\Games" },
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "save_settings",
        expect.objectContaining({
          settings: expect.objectContaining({
            dataDirectory: "D:\\Games",
            downloadConcurrency: 8,
            writeConcurrency: 8,
            theme: "dark",
          }),
        }),
      );
    });
  });

  it("saves a global MiB/s limit and removes it when the input is cleared", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));
    const limit = screen.getByLabelText("Global download speed limit (MiB/s)");
    fireEvent.change(limit, { target: { value: "3" } });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "save_settings",
        expect.objectContaining({
          settings: expect.objectContaining({ downloadSpeedLimitMiB: 3 }),
        }),
      ),
    );
    fireEvent.change(limit, { target: { value: "" } });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "save_settings",
        expect.objectContaining({
          settings: expect.objectContaining({ downloadSpeedLimitMiB: null }),
        }),
      ),
    );
  });

  it("rejects a zero speed limit instead of saving an invalid setting", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));
    fireEvent.change(
      screen.getByLabelText("Global download speed limit (MiB/s)"),
      { target: { value: "0" } },
    );
    expect(
      await screen.findByText(
        "Download speed limit must be a positive whole number of MiB/s.",
      ),
    ).toBeVisible();
    expect(invoke).not.toHaveBeenCalledWith(
      "save_settings",
      expect.objectContaining({
        settings: expect.objectContaining({ downloadSpeedLimitMiB: 0 }),
      }),
    );
  });
});
