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

  it("adds two short details per card without repeating the description", () => {
    render(<StepSource value="scratch" onChange={() => {}} />);

    for (const detail of [
      "Vanilla, Fabric, NeoForge, Forge or Quilt",
      "Starts empty, add mods later",
      "Mods come already selected",
      "Choose any published pack version",
      "The format is detected for you",
      "Your original file is left untouched",
    ]) {
      expect(screen.getByText(detail)).toBeVisible();
    }
    for (const card of screen.getAllByRole("button")) {
      expect(card.querySelectorAll("li")).toHaveLength(2);
    }
  });

  it("animates the selection mark in and out instead of mounting it", () => {
    const { rerender } = render(
      <StepSource value="scratch" onChange={() => {}} />,
    );
    const marks = () =>
      screen
        .getAllByRole("button")
        .map((card) =>
          card
            .querySelector("[data-slot='selected-mark']")
            ?.getAttribute("data-selected"),
        );

    expect(marks()).toEqual(["true", "false", "false"]);
    rerender(<StepSource value="file" onChange={() => {}} />);
    expect(marks()).toEqual(["false", "false", "true"]);
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
