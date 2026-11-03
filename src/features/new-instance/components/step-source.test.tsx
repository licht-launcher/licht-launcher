import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { StepSource } from "@/features/new-instance/components/step-source";

describe("StepSource", () => {
  it("marks only the chosen source as selected", () => {
    render(<StepSource value="scratch" onChange={() => {}} />);

    expect(
      screen.getByRole("button", { name: /From scratch/ }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      screen.getByRole("button", { name: /From a modpack/ }),
    ).toHaveAttribute("aria-pressed", "false");
    expect(screen.getAllByText("Selected")).toHaveLength(1);
  });

  it("highlights the key words of each description", () => {
    render(<StepSource value="scratch" onChange={() => {}} />);

    for (const word of ["platform", "game version", "Modrinth"]) {
      expect(screen.getByText(word, { selector: "strong" })).toBeVisible();
    }
    for (const extension of [".mrpack", ".zip"]) {
      expect(screen.getByText(extension, { selector: "code" })).toBeVisible();
    }
  });

  it("keeps the emphasis light instead of bold", () => {
    render(<StepSource value="scratch" onChange={() => {}} />);

    const word = screen.getByText("platform", { selector: "strong" });
    expect(word).toHaveClass("font-medium");
    expect(word).not.toHaveClass("font-semibold");
    expect(
      screen
        .getByRole("button", { name: /From scratch/ })
        .querySelector(".font-brand"),
    ).toBeNull();
  });

  it("reports the source the user picks", () => {
    const onChange = vi.fn();
    render(<StepSource value="scratch" onChange={onChange} />);

    fireEvent.click(screen.getByRole("button", { name: /Import a pack file/ }));
    expect(onChange).toHaveBeenCalledWith("file");
    fireEvent.click(screen.getByRole("button", { name: /From a modpack/ }));
    expect(onChange).toHaveBeenCalledWith("modpack");
  });
});
