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

async function selectRam(label: string) {
  const select = await screen.findByRole("combobox", { name: "Simulated RAM" });
  fireEvent.keyDown(select, { key: "ArrowDown" });
  fireEvent.click(await screen.findByRole("option", { name: label }));
  await waitFor(() => expect(select).toHaveTextContent(label));
}

describe("Memory Allocation preview", () => {
  beforeEach(() => {
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
  });

  it("shows RAM choices only when simulation is enabled and starts at 32 GiB", async () => {
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    const testing = within(screen.getByRole("region", { name: "Testing" }));
    const simulate = testing.getByRole("switch", { name: "Simulate memory" });
    expect(simulate).not.toBeChecked();
    expect(
      testing.queryByRole("combobox", { name: "Simulated RAM" }),
    ).not.toBeInTheDocument();
    fireEvent.click(simulate);
    const select = await testing.findByRole("combobox", {
      name: "Simulated RAM",
    });
    expect(simulate).toBeChecked();
    expect(select).toHaveTextContent("32 GiB");
    fireEvent.keyDown(select, { key: "ArrowDown" });
    const options = within(await screen.findByRole("listbox")).getAllByRole(
      "option",
    );
    expect(options.map((option) => option.textContent)).toEqual([
      "4 GiB",
      "8 GiB",
      "16 GiB",
      "32 GiB",
      "64 GiB",
      "128 GiB",
    ]);
  });

  it("previews larger RAM and allocations without persisting them, then restores real values", async () => {
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    fireEvent.click(screen.getByRole("switch", { name: "Simulate memory" }));
    await selectRam("64 GiB");
    fireEvent.click(screen.getByRole("button", { name: "Instances" }));
    expect(screen.getByText("Memory Allocation")).toBeVisible();
    expect(screen.getByText(/Preview: 64 GiB RAM/)).toBeVisible();
    const input = screen.getByRole("spinbutton", {
      name: "Memory Allocation (MB)",
    });
    const slider = screen.getByRole("slider", { name: "Memory" });
    expect(slider).toHaveAttribute("aria-valuemax", "65536");
    fireEvent.change(input, { target: { value: "50001" } });
    await waitFor(() =>
      expect(slider).toHaveAttribute("aria-valuenow", "50001"),
    );
    fireEvent.click(screen.getByRole("switch", { name: "Fullscreen" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("save_settings", {
        settings: expect.objectContaining({
          fullscreen: true,
          maxMemoryMb: snapshot.settings.maxMemoryMb,
        }),
      }),
    );
    expect(invoke).not.toHaveBeenCalledWith("save_settings", {
      settings: expect.objectContaining({ maxMemoryMb: 50001 }),
    });
    fireEvent.click(screen.getByRole("button", { name: "Developer Tools" }));
    expect(
      screen.getByRole("combobox", { name: "Simulated RAM" }),
    ).toHaveTextContent("64 GiB");
    fireEvent.click(screen.getByRole("button", { name: "Instances" }));
    expect(
      await screen.findByRole("spinbutton", { name: "Memory Allocation (MB)" }),
    ).toHaveValue(50001);
    fireEvent.click(screen.getByRole("button", { name: "Developer Tools" }));
    fireEvent.click(screen.getByRole("switch", { name: "Simulate memory" }));
    await waitFor(() =>
      expect(
        screen.queryByRole("combobox", { name: "Simulated RAM" }),
      ).not.toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Instances" }));
    expect(
      screen.getByRole("spinbutton", { name: "Memory Allocation (MB)" }),
    ).toHaveValue(snapshot.settings.maxMemoryMb);
    expect(screen.getByRole("slider", { name: "Memory" })).toHaveAttribute(
      "aria-valuemax",
      String(snapshot.memoryMaximumMb),
    );
    expect(screen.queryByText(/Preview:.*GiB RAM/)).not.toBeInTheDocument();
  });

  it("limits a smaller-machine preview without overwriting the real allocation", async () => {
    mockSettingsCommands((command) =>
      command === "get_settings"
        ? Promise.resolve({
            ...snapshot,
            settings: { ...snapshot.settings, maxMemoryMb: 12001 },
          })
        : undefined,
    );
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", { name: "Developer Tools" }),
    );
    fireEvent.click(screen.getByRole("switch", { name: "Simulate memory" }));
    await selectRam("4 GiB");
    fireEvent.click(screen.getByRole("button", { name: "Instances" }));
    expect(
      screen.getByRole("spinbutton", { name: "Memory Allocation (MB)" }),
    ).toHaveValue(4096);
    expect(screen.getByRole("slider", { name: "Memory" })).toHaveAttribute(
      "aria-valuemax",
      "4096",
    );
    fireEvent.click(screen.getByRole("button", { name: "Developer Tools" }));
    fireEvent.click(screen.getByRole("switch", { name: "Simulate memory" }));
    await waitFor(() =>
      expect(
        screen.queryByRole("combobox", { name: "Simulated RAM" }),
      ).not.toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Instances" }));
    expect(
      screen.getByRole("spinbutton", { name: "Memory Allocation (MB)" }),
    ).toHaveValue(12001);
    expect(invoke).not.toHaveBeenCalledWith("save_settings", expect.anything());
  });
});
