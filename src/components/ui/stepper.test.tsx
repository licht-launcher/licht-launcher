import { fireEvent, render, screen } from "@testing-library/react";
import { BoxIcon } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { Stepper } from "@/components/ui/stepper";

const steps = [
  { label: "Source", icon: BoxIcon },
  { label: "Platform", icon: BoxIcon },
  { label: "Confirm", icon: BoxIcon },
];

describe("Stepper", () => {
  it("marks the current step", () => {
    render(<Stepper currentIndex={1} steps={steps} />);

    const list = screen.getByRole("list", { name: "Wizard progress" });
    expect(list).toBeInTheDocument();
    expect(
      screen.getByText("Platform").closest("[aria-current]"),
    ).toHaveAttribute("aria-current", "step");
  });

  it("shows a check for finished steps and a number for the rest", () => {
    render(<Stepper currentIndex={1} steps={steps} />);

    expect(screen.getAllByTestId("step-done")).toHaveLength(1);
    expect(screen.getByText("2")).toBeVisible();
    expect(screen.getByText("3")).toBeVisible();
  });

  it("lets the user go back to a finished step only", () => {
    const onSelect = vi.fn();
    render(<Stepper currentIndex={2} steps={steps} onSelect={onSelect} />);

    fireEvent.click(screen.getByRole("button", { name: /Source/ }));
    expect(onSelect).toHaveBeenCalledWith(0);
    fireEvent.click(screen.getByRole("button", { name: /Platform/ }));
    expect(onSelect).toHaveBeenCalledWith(1);
    expect(
      screen.queryByRole("button", { name: /Confirm/ }),
    ).not.toBeInTheDocument();
    expect(onSelect).toHaveBeenCalledTimes(2);
  });

  it("renders plain steps when nothing handles selection", () => {
    render(<Stepper currentIndex={2} steps={steps} />);

    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });
});
