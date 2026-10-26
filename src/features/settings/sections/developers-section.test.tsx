import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { STORAGE_LOADING_PREVIEW_EVENT } from "@/features/downloads/hooks/use-storage-loading-preview";
import { DevelopersSection } from "@/features/settings/sections/developers-section";
import {
  DEV_CLEAR_SIMULATED_UPDATE_EVENT,
  DEV_SIMULATE_UPDATE_EVENT,
} from "@/features/updater/components/update-check";

const { openPreview, restartPreview } = vi.hoisted(() => ({
  openPreview: vi.fn(),
  restartPreview: vi.fn(),
}));
vi.mock("@/lib/commands", () => ({
  openStartupPreview: openPreview,
  restartStartupPreview: restartPreview,
}));

function view() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <DevelopersSection />
    </QueryClientProvider>,
  );
}

describe("DevelopersSection", () => {
  beforeEach(() => {
    openPreview.mockReset();
    restartPreview.mockReset();
  });
  afterEach(() => {
    document.getElementById("developer-acrylic-preview")?.remove();
  });

  it("previews opacity, lightness and hue with keyboard controls", () => {
    view();
    for (const [label, token, initial, unit] of [
      ["Sidebar opacity", "opacity", 60, "%"],
      ["Sidebar lightness", "lightness", 15, "%"],
      ["Sidebar hue", "hue", 260, ""],
    ] as const) {
      const slider = screen.getByRole("slider", { name: label });
      expect(slider).toHaveAttribute("aria-valuenow", String(initial));
      fireEvent.keyDown(slider, { key: "ArrowRight" });
      expect(slider).toHaveAttribute("aria-valuenow", String(initial + 1));
      expect(
        getComputedStyle(document.documentElement)
          .getPropertyValue(`--acrylic-${token}`)
          .trim(),
      ).toBe(`${initial + 1}${unit}`);
    }
    expect(screen.getByText("61%")).toBeVisible();
    expect(screen.getByText("16%")).toBeVisible();
    expect(screen.getByText("261°")).toBeVisible();
  });

  it("keeps previews when reopening the section and resets all adjustments", () => {
    const first = view();
    fireEvent.keyDown(screen.getByRole("slider", { name: "Sidebar opacity" }), {
      key: "End",
    });
    fireEvent.keyDown(
      screen.getByRole("slider", { name: "Sidebar lightness" }),
      { key: "ArrowRight" },
    );
    fireEvent.keyDown(screen.getByRole("slider", { name: "Sidebar hue" }), {
      key: "Home",
    });
    first.unmount();
    view();
    expect(
      screen.getByRole("slider", { name: "Sidebar opacity" }),
    ).toHaveAttribute("aria-valuenow", "100");
    expect(
      screen.getByRole("slider", { name: "Sidebar lightness" }),
    ).toHaveAttribute("aria-valuenow", "16");
    expect(screen.getByRole("slider", { name: "Sidebar hue" })).toHaveAttribute(
      "aria-valuenow",
      "0",
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Reset Acrylic preview" }),
    );
    expect(
      screen.getByRole("slider", { name: "Sidebar opacity" }),
    ).toHaveAttribute("aria-valuenow", "60");
    expect(
      screen.getByRole("slider", { name: "Sidebar lightness" }),
    ).toHaveAttribute("aria-valuenow", "15");
    expect(screen.getByRole("slider", { name: "Sidebar hue" })).toHaveAttribute(
      "aria-valuenow",
      "260",
    );
    expect(
      getComputedStyle(document.documentElement).getPropertyValue(
        "--acrylic-opacity",
      ),
    ).toBe("");
  });
  it("toggles update simulation", () => {
    const simulateListener = vi.fn();
    const clearListener = vi.fn();
    window.addEventListener(DEV_SIMULATE_UPDATE_EVENT, simulateListener);
    window.addEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, clearListener);

    view();
    const switchControl = screen.getByRole("switch", {
      name: "Simulate available update",
    });

    fireEvent.click(switchControl);
    expect(simulateListener).toHaveBeenCalledOnce();
    expect(switchControl).toBeChecked();

    fireEvent.click(switchControl);
    expect(clearListener).toHaveBeenCalledOnce();
    expect(switchControl).not.toBeChecked();

    window.removeEventListener(DEV_SIMULATE_UPDATE_EVENT, simulateListener);
    window.removeEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, clearListener);
  });

  it("requests a repeatable Launcher Data loading preview", () => {
    const listener = vi.fn();
    window.addEventListener(STORAGE_LOADING_PREVIEW_EVENT, listener);
    view();
    const button = screen.getByRole("button", {
      name: "Test Launcher Data loading",
    });
    fireEvent.click(button);
    fireEvent.click(button);
    expect(listener).toHaveBeenCalledTimes(2);
    window.removeEventListener(STORAGE_LOADING_PREVIEW_EVENT, listener);
  });

  it("does not expose the storage preview in production", () => {
    vi.stubEnv("DEV", false);
    try {
      view();
      expect(
        screen.queryByRole("button", { name: "Test Launcher Data loading" }),
      ).not.toBeInTheDocument();
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it("opens the real startup preview with a predefined isolated scenario", async () => {
    openPreview.mockResolvedValueOnce(undefined);
    view();
    fireEvent.click(
      screen.getByRole("combobox", { name: "Startup test scenario" }),
    );
    fireEvent.click(
      await screen.findByRole("option", {
        name: "Migration with temporary fixtures",
      }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Open startup preview" }),
    );
    await waitFor(() => expect(openPreview).toHaveBeenCalledWith("migration"));
    expect(restartPreview).not.toHaveBeenCalled();
  });

  it("requests a startup test restart and reports launch failures", async () => {
    restartPreview.mockRejectedValueOnce(new Error("could not restart"));
    view();
    fireEvent.click(
      screen.getByRole("button", { name: "Restart and test startup" }),
    );
    await waitFor(() =>
      expect(restartPreview).toHaveBeenCalledWith("noUpdate"),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "could not restart",
    );
  });
});
