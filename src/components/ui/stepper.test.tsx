import { render, screen } from "@testing-library/react";
import { BoxIcon } from "lucide-react";
import { describe, expect, it } from "vitest";

import { Stepper } from "@/components/ui/stepper";

describe("Stepper", () => {
  it("marks the current step", () => {
    render(
      <Stepper
        currentIndex={1}
        steps={[
          { label: "Source", icon: BoxIcon },
          { label: "Platform", icon: BoxIcon },
          { label: "Confirm", icon: BoxIcon },
        ]}
      />,
    );

    const list = screen.getByRole("list", { name: "Wizard progress" });
    expect(list).toBeInTheDocument();
    expect(
      screen.getByText("Platform").closest("[aria-current]"),
    ).toHaveAttribute("aria-current", "step");
  });
});
