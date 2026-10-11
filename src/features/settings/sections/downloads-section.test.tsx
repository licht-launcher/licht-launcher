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

  it("saves the application directory and says the change waits for a restart", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Downloads" }));

    expect(
      screen.getByText(/This change applies after restart\./),
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
});
