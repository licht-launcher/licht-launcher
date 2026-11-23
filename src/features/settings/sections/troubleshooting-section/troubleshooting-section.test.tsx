import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { formatBuildDiagnostics } from "@/api/build-info";
import { buildDiagnostics } from "@/features/settings/fixtures/build-diagnostics";
import {
  invoke,
  logsFolder,
  mockSettingsCommands,
  renderSettings,
} from "@/features/settings/fixtures/render-settings";

vi.mock("@tauri-apps/api/core", async () => {
  const { invoke: mocked } = await import(
    "@/features/settings/fixtures/invoke"
  );
  return { invoke: mocked };
});

async function openTroubleshooting() {
  await renderSettings();
  const navigation = within(
    await screen.findByRole("navigation", { name: "Settings sections" }),
  );
  fireEvent.click(
    await navigation.findByRole("button", { name: "Troubleshooting" }),
  );
  expect(
    await screen.findByRole("heading", { level: 2, name: "Troubleshooting" }),
  ).toBeVisible();
}

function stubClipboard(writeText = vi.fn().mockResolvedValue(undefined)) {
  vi.stubGlobal("navigator", {
    userAgent: navigator.userAgent,
    clipboard: { writeText },
  });
  return writeText;
}

describe("TroubleshootingSection", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });

  afterEach(() => {
    toast.dismiss();
    vi.unstubAllGlobals();
  });

  it("shows the log folder and opens it", async () => {
    await openTroubleshooting();
    const logs = within(screen.getByRole("region", { name: "Logs" }));

    expect(await logs.findByText(logsFolder)).toBeVisible();
    fireEvent.click(logs.getByRole("button", { name: "Open logs folder" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("open_logs_folder"),
    );
  });

  it("reports a log folder that cannot be opened", async () => {
    const message =
      "The log folder does not exist yet. Restart the launcher to create it.";
    mockSettingsCommands((command) =>
      command === "open_logs_folder" ? Promise.reject(message) : undefined,
    );
    await openTroubleshooting();

    fireEvent.click(
      await screen.findByRole("button", { name: "Open logs folder" }),
    );

    expect(await screen.findByText(message)).toBeVisible();
  });

  it("explains when the log folder cannot be located", async () => {
    mockSettingsCommands((command) =>
      command === "get_logs_folder"
        ? Promise.reject("Could not resolve the launcher data directory")
        : undefined,
    );
    await openTroubleshooting();
    const logs = within(screen.getByRole("region", { name: "Logs" }));

    expect(
      await logs.findByText(
        "Log folder unavailable: Could not resolve the launcher data directory",
      ),
    ).toBeVisible();
  });

  it("shows and copies the build and system information", async () => {
    const writeText = stubClipboard();
    await openTroubleshooting();
    const build = within(
      screen.getByRole("region", { name: "Build and system" }),
    );
    const formatted = formatBuildDiagnostics(buildDiagnostics);

    for (const line of formatted.split("\n")) {
      expect(await build.findByText(line)).toBeVisible();
    }
    fireEvent.click(build.getByRole("button", { name: "Copy" }));

    await waitFor(() => expect(writeText).toHaveBeenCalledWith(formatted));
    expect(await screen.findByText("Build information copied.")).toBeVisible();
  });

  it("explains when the build information cannot be read", async () => {
    mockSettingsCommands((command) =>
      command === "get_build_diagnostics"
        ? Promise.reject(new Error("Runtime information unavailable"))
        : undefined,
    );
    await openTroubleshooting();
    const build = within(
      screen.getByRole("region", { name: "Build and system" }),
    );

    expect(
      await build.findByText(
        "Build information unavailable: Runtime information unavailable",
      ),
    ).toBeVisible();
    expect(build.getByRole("button", { name: "Copy" })).toBeDisabled();
  });

  it("copies the build information before opening the report page", async () => {
    const order: string[] = [];
    const writeText = stubClipboard(
      vi.fn().mockImplementation(() => {
        order.push("copy");
        return Promise.resolve();
      }),
    );
    mockSettingsCommands((command) => {
      if (command === "open_feedback") {
        order.push("open");
        return Promise.resolve();
      }
      return undefined;
    });
    await openTroubleshooting();

    fireEvent.click(
      await screen.findByRole("button", { name: "Report a problem" }),
    );

    await waitFor(() => expect(order).toEqual(["copy", "open"]));
    expect(writeText).toHaveBeenCalledWith(
      formatBuildDiagnostics(buildDiagnostics),
    );
    expect(
      await screen.findByText(
        "Build information copied. Paste it into your report.",
      ),
    ).toBeVisible();
  });

  it("does not open the report page when the information cannot be copied", async () => {
    stubClipboard(
      vi.fn().mockRejectedValue(new Error("Clipboard unavailable")),
    );
    await openTroubleshooting();

    fireEvent.click(
      await screen.findByRole("button", { name: "Report a problem" }),
    );

    expect(await screen.findByText("Clipboard unavailable")).toBeVisible();
    expect(invoke).not.toHaveBeenCalledWith("open_feedback");
  });
});
