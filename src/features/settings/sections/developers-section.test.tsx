import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { DevelopersSection } from "@/features/settings/sections/developers-section";
import {
  DEV_CLEAR_SIMULATED_UPDATE_EVENT,
  DEV_SIMULATE_UPDATE_EVENT,
} from "@/features/updater/components/update-check";

describe("DevelopersSection", () => {
  it("toggles update simulation", () => {
    const simulateListener = vi.fn();
    const clearListener = vi.fn();
    window.addEventListener(DEV_SIMULATE_UPDATE_EVENT, simulateListener);
    window.addEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, clearListener);

    render(<DevelopersSection />);
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
});
