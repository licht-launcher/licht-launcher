import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import { Switch } from "@/components/ui/switch";

function ControlledSwitch() {
  const [checked, setChecked] = useState(false);
  return (
    <Switch
      aria-label="Full screen"
      checked={checked}
      onCheckedChange={setChecked}
    />
  );
}

describe("Switch", () => {
  it("toggles when clicked", () => {
    render(<ControlledSwitch />);

    const toggle = screen.getByRole("switch", { name: "Full screen" });
    expect(toggle).toHaveAttribute("aria-checked", "false");

    fireEvent.click(toggle);

    expect(toggle).toHaveAttribute("aria-checked", "true");
  });
});
