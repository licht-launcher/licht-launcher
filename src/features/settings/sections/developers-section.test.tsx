import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

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
