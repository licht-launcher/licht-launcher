import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { MemorySlider } from "@/features/settings/components/memory-slider";

describe("MemorySlider", () => {
  it("uses actual MB values and moves in 64 MB increments between reference marks", () => {
    const onChange = vi.fn();
    render(
      <MemorySlider
        valueMb={2048}
        minimumMb={512}
        maximumMb={8192}
        recommendedMb={4096}
        totalMb={8192}
        stepMb={64}
        onChange={onChange}
      />,
    );

    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-valuenow", "2048");
    expect(slider).toHaveAttribute("aria-valuemin", "512");
    expect(slider).toHaveAttribute("aria-valuemax", "8192");
    expect(screen.getByRole("button", { name: "2GB" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    fireEvent.click(screen.getByRole("button", { name: "4GB" }));
    expect(onChange).toHaveBeenCalledWith(4096);

    fireEvent.keyDown(slider, { key: "ArrowRight" });
    expect(onChange).toHaveBeenLastCalledWith(2112);
  });

  it("represents exact typed allocations and reaches the RAM endpoint even outside the 64 MB grid", () => {
    const onChange = vi.fn();
    render(
      <MemorySlider
        valueMb={5001}
        minimumMb={512}
        maximumMb={32701}
        totalMb={32701}
        recommendedMb={4096}
        stepMb={64}
        onChange={onChange}
      />,
    );
    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-valuenow", "5001");
    fireEvent.keyDown(slider, { key: "End" });
    expect(onChange).toHaveBeenLastCalledWith(32701);
    fireEvent.keyDown(slider, { key: "Home" });
    expect(onChange).toHaveBeenLastCalledWith(512);
  });
});
