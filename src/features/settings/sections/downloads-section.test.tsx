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
    const network = within(screen.getByRole("region", { name: "Network" }));
    const storage = within(screen.getByRole("region", { name: "Storage" }));
    expect(
      network.getByRole("switch", { name: "Limit download speed" }),
    ).not.toBeChecked();
    expect(
      network.queryByLabelText("Speed limit (kB/s)"),
    ).not.toBeInTheDocument();
    expect(network.queryByRole("slider")).not.toBeInTheDocument();
    expect(storage.queryByRole("slider")).not.toBeInTheDocument();
    expect(storage.getByLabelText("Application directory")).toBeInTheDocument();
    expect(
      await storage.findByRole("img", { name: /Launcher storage/ }),
    ).toBeVisible();
    expect(storage.getByText("Natives")).toBeVisible();
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

  it("saves kB/s as bytes/s, shows MiB/s and removes the limit when cleared", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Limit download speed" }),
    );
    const limit = screen.getByLabelText("Speed limit (kB/s)");
    expect(limit).toHaveAccessibleDescription(
      "Enter the maximum download speed in kilobytes per second.",
    );
    fireEvent.change(limit, { target: { value: "1000" } });
    expect(screen.getByText("Downloads limited to 0.95 MiB/s")).toBeVisible();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "save_settings",
        expect.objectContaining({
          settings: expect.objectContaining({
            downloadSpeedLimitBytesPerSecond: 1_000_000,
          }),
        }),
      ),
    );
    fireEvent.change(limit, { target: { value: "" } });
    expect(screen.queryByText(/Downloads limited to/)).not.toBeInTheDocument();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "save_settings",
        expect.objectContaining({
          settings: expect.objectContaining({
            downloadSpeedLimitBytesPerSecond: null,
          }),
        }),
      ),
    );
  });

  it.each(["0", "-5"])(
    "keeps the input empty for %s without saving an invalid limit",
    async (value) => {
      await renderSettings();
      fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));
      const toggle = screen.getByRole("switch", {
        name: "Limit download speed",
      });
      fireEvent.click(toggle);
      const input = screen.getByLabelText("Speed limit (kB/s)");
      fireEvent.change(input, { target: { value } });
      expect(input).toHaveDisplayValue("");
      expect(toggle).toBeChecked();
      expect(
        screen.queryByText(/Downloads limited to/),
      ).not.toBeInTheDocument();
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith(
          "save_settings",
          expect.objectContaining({
            settings: expect.objectContaining({
              downloadSpeedLimitBytesPerSecond: null,
            }),
          }),
        ),
      );
      expect(invoke).not.toHaveBeenCalledWith(
        "save_settings",
        expect.objectContaining({
          settings: expect.objectContaining({
            downloadSpeedLimitBytesPerSecond: 0,
          }),
        }),
      );
    },
  );

  it("hides the input and removes a configured limit when switched off", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));
    const toggle = screen.getByRole("switch", { name: "Limit download speed" });
    fireEvent.click(toggle);
    fireEvent.change(screen.getByLabelText("Speed limit (kB/s)"), {
      target: { value: "1000" },
    });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "save_settings",
        expect.objectContaining({
          settings: expect.objectContaining({
            downloadSpeedLimitBytesPerSecond: 1_000_000,
          }),
        }),
      ),
    );
    fireEvent.click(toggle);
    expect(
      screen.queryByLabelText("Speed limit (kB/s)"),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/Downloads limited to/)).not.toBeInTheDocument();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "save_settings",
        expect.objectContaining({
          settings: expect.objectContaining({
            downloadSpeedLimitBytesPerSecond: null,
          }),
        }),
      ),
    );
    fireEvent.click(toggle);
    expect(screen.getByLabelText("Speed limit (kB/s)")).toHaveDisplayValue("");
  });

  it("loads an existing limit with its switch enabled and preserves its byte rate", async () => {
    mockSettingsCommands((command) =>
      command === "get_settings"
        ? Promise.resolve({
            ...snapshot,
            settings: {
              ...snapshot.settings,
              downloadSpeedLimitBytesPerSecond: 1_048_576,
            },
          })
        : undefined,
    );
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));
    expect(
      screen.getByRole("switch", { name: "Limit download speed" }),
    ).toBeChecked();
    expect(screen.getByLabelText("Speed limit (kB/s)")).toHaveValue(1048.576);
    expect(screen.getByText("Downloads limited to 1 MiB/s")).toBeVisible();
  });

  it("rejects rates with unsupported precision instead of saving an invalid setting", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Limit download speed" }),
    );
    fireEvent.change(screen.getByLabelText("Speed limit (kB/s)"), {
      target: { value: "0.0001" },
    });
    expect(
      await screen.findByText(
        "Download speed limit must be a positive number of kB/s with at most three decimal places.",
      ),
    ).toBeVisible();
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });
});
