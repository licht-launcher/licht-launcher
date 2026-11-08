import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import { Slider } from "@/components/ui/slider";

function ControlledSlider() {
  const [value, setValue] = useState([4]);
  return (
    <Slider
      aria-label="Concurrency"
      min={1}
      max={16}
      step={1}
      value={value}
      onValueChange={setValue}
    />
  );
}

describe("Slider", () => {
  it("moves with the arrow keys", () => {
    render(<ControlledSlider />);

    const slider = screen.getByRole("slider");
    expect(slider).toHaveAttribute("aria-valuenow", "4");

    fireEvent.keyDown(slider, { key: "ArrowRight" });

    expect(slider).toHaveAttribute("aria-valuenow", "5");
  });
});
