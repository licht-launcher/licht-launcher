import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { buildDiagnostics } from "@/features/settings/fixtures/build-diagnostics";
import {
  mockSettingsCommands,
  renderSettings,
} from "@/features/settings/fixtures/render-settings";
import { formatBuildDiagnostics } from "@/lib/build-info";

vi.mock("@tauri-apps/api/core", async () => {
  const { invoke: mocked } = await import(
    "@/features/settings/fixtures/invoke"
  );
  return { invoke: mocked };
});

describe("SettingsSidebar", () => {
  beforeEach(() => {
    mockSettingsCommands();
  });
  afterEach(() => vi.unstubAllGlobals());

  it("renders Runtime with a processor icon and separates Developer Tools from the main sections", async () => {
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });
    const navigation = within(
      screen.getByRole("navigation", { name: "Settings sections" }),
    );
    const javaIcon = navigation
      .getByRole("button", { name: "Runtime" })
      .querySelector("svg");
    expect(javaIcon).toHaveClass("lucide-cpu");
    expect(javaIcon).toHaveAttribute("aria-hidden", "true");
    expect(javaIcon).toHaveClass("size-4");
    expect(
      navigation.queryByRole("button", { name: "Developer Tools" }),
    ).not.toBeInTheDocument();
    const developers = within(
      screen.getByRole("navigation", { name: "Developer settings" }),
    );
    fireEvent.click(
      developers.getByRole("button", { name: "Developer Tools" }),
    );
    expect(
      await screen.findByRole("heading", { name: "Developer Tools" }),
    ).toBeVisible();
    expect(
      developers.getByRole("button", { name: "Developer Tools" }),
    ).toHaveAttribute("aria-current", "page");
  });

  it("copies the build, target system and revision from the footer", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", {
      userAgent: navigator.userAgent,
      clipboard: { writeText },
    });
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Copy build and system information",
      }),
    );
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith(
        formatBuildDiagnostics(buildDiagnostics),
      ),
    );
    await waitFor(() =>
      expect(screen.getByRole("tooltip")).toHaveTextContent("Copied!"),
    );
    expect(
      screen.queryByText("Build information copied."),
    ).not.toBeInTheDocument();
  });

  it("reports a failed clipboard write", async () => {
    const writeText = vi
      .fn()
      .mockRejectedValue(new Error("Clipboard unavailable"));
    vi.stubGlobal("navigator", {
      userAgent: navigator.userAgent,
      clipboard: { writeText },
    });
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Copy build and system information",
      }),
    );
    await waitFor(() =>
      expect(screen.getByRole("tooltip")).toHaveTextContent(
        "Clipboard unavailable",
      ),
    );
  });

  it("reports failed native diagnostics without copying incomplete information", async () => {
    const writeText = vi.fn();
    vi.stubGlobal("navigator", {
      userAgent: navigator.userAgent,
      clipboard: { writeText },
    });
    mockSettingsCommands((command) =>
      command === "get_build_diagnostics"
        ? Promise.reject(new Error("Runtime information unavailable"))
        : undefined,
    );
    await renderSettings();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Copy build and system information",
      }),
    );
    await waitFor(() =>
      expect(screen.getByRole("tooltip")).toHaveTextContent(
        "Runtime information unavailable",
      ),
    );
    expect(writeText).not.toHaveBeenCalled();
  });

  it("keeps an accessible dialog name and navigates without the app footer", async () => {
    await renderSettings();
    await screen.findByRole("button", { name: "Dark" });

    expect(
      screen.getByRole("dialog", { name: "Settings" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Settings" })).toHaveClass(
      "sr-only",
    );
    expect(screen.queryByText("Licht Launcher")).not.toBeInTheDocument();
    expect(screen.queryByText("0.3.0 · Rose")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "GitHub repository" }),
    ).not.toBeInTheDocument();

    const navigation = screen.getByRole("navigation", {
      name: "Settings sections",
    });
    expect(
      within(navigation).queryByRole("button", { name: "Account" }),
    ).not.toBeInTheDocument();
    expect(
      within(navigation).queryByRole("button", { name: "Appearance" }),
    ).not.toBeInTheDocument();
    fireEvent.click(
      within(navigation).getByRole("button", { name: "Downloads" }),
    );
    fireEvent.click(
      within(navigation).getByRole("button", { name: "General" }),
    );
    expect(
      await screen.findByRole("heading", { name: "General" }),
    ).toBeVisible();
    expect(
      within(navigation).getByRole("button", { name: "General" }),
    ).toHaveAttribute("aria-current", "page");
  });
});
