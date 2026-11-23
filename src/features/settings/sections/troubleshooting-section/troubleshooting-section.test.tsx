import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  buildDiagnosticsEntries,
  formatBuildDiagnostics,
} from "@/api/build-info";
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

  it("puts problem reports first, then logs, then build information", async () => {
    await openTroubleshooting();
    const [report, logs, build] = [
      "Report a problem",
      "Logs",
      "Build and system",
    ].map((name) => screen.getByRole("region", { name }));

    expect(
      report.compareDocumentPosition(logs) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      logs.compareDocumentPosition(build) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(within(report).getByText("Open an issue on GitHub")).toBeVisible();
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

    for (const entry of buildDiagnosticsEntries(buildDiagnostics)) {
      const term = await build.findByText(entry.label, { selector: "dt" });
      expect(term.nextElementSibling?.tagName).toBe("DD");
      expect(term.nextElementSibling).toHaveTextContent(entry.value);
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

  it("opens the report page without copying anything", async () => {
    const writeText = stubClipboard();
    await openTroubleshooting();
    const button = await screen.findByRole("button", {
      name: "Report a problem",
    });

    expect(button.lastElementChild).toHaveClass("lucide-external-link");
    fireEvent.click(button);

    await waitFor(() => expect(invoke).toHaveBeenCalledWith("open_feedback"));
    expect(writeText).not.toHaveBeenCalled();
  });

  it("reports a report page that cannot be opened", async () => {
    mockSettingsCommands((command) =>
      command === "open_feedback"
        ? Promise.reject("No browser is available")
        : undefined,
    );
    await openTroubleshooting();

    fireEvent.click(
      await screen.findByRole("button", { name: "Report a problem" }),
    );

    expect(await screen.findByText("No browser is available")).toBeVisible();
  });
});
