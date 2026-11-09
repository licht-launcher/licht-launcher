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

describe("InstancesSection", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  it("disables width and height while full screen is on", async () => {
    await renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Instances" }));

    const width = screen.getByLabelText("Window width");
    const height = screen.getByLabelText("Window height");
    expect(width).toBeEnabled();
    expect(height).toBeEnabled();

    fireEvent.click(screen.getByRole("switch", { name: "Fullscreen" }));

    expect(width).toBeDisabled();
    expect(height).toBeDisabled();
    expect(width).toHaveAttribute("placeholder", "854");
    expect(height).toHaveAttribute("placeholder", "480");
    expect(
      screen.getByText(
        "Applied to every instance unless that instance customizes the setting.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "The game starts in fullscreen. Width and height are not used.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Window width in pixels.")).toBeInTheDocument();
    expect(screen.getByText("Window height in pixels.")).toBeInTheDocument();
    expect(
      screen.getByText("Memory allocation. The minimum stays 512 MB."),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("Java arguments")).not.toBeInTheDocument();
  });

  it("synchronizes exact memory input and 64 MB slider changes up to all available RAM", async () => {
    mockSettingsCommands((command, args) => {
      if (command === "get_settings")
        return Promise.resolve({
          ...snapshot,
          memoryMaximumMb: 32701,
          memoryTotalMb: 32701,
        });
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
    fireEvent.click(await screen.findByRole("button", { name: "Instances" }));
    const input = screen.getByRole("spinbutton", {
      name: "Memory Allocation (MB)",
    });
    const slider = screen.getByRole("slider", { name: "Memory" });
    fireEvent.change(input, { target: { value: "20001" } });
    expect(slider).toHaveAttribute("aria-valuenow", "20001");
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ maxMemoryMb: 20001 }),
      }),
    );
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(input).toHaveValue(20065);
    fireEvent.keyDown(slider, { key: "End" });
    expect(input).toHaveValue(32701);
    expect(slider).toHaveAttribute("aria-valuemax", "32701");
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({ maxMemoryMb: 32701 }),
      }),
    );
  });
});
