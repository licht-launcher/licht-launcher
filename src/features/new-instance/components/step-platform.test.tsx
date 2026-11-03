import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { StepPlatform } from "@/features/new-instance/components/step-platform";

describe("StepPlatform", () => {
  it("asks for the instance name next to the platform", () => {
    const onNameChange = vi.fn();
    render(
      <StepPlatform
        value="vanilla"
        name="Survival"
        onChange={() => {}}
        onNameChange={onNameChange}
      />,
    );

    const name = screen.getByLabelText("Instance Name");
    expect(name).toHaveValue("Survival");
    fireEvent.change(name, { target: { value: "Hardcore" } });
    expect(onNameChange).toHaveBeenCalledWith("Hardcore");
  });

  it("separates the name and the platform into titled sections", () => {
    render(
      <StepPlatform
        value="vanilla"
        name=""
        onChange={() => {}}
        onNameChange={() => {}}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "Name your instance" }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "Choose a platform" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Vanilla is the official game. Loaders add mod support.",
      ),
    ).toBeVisible();
  });

  it("reports the chosen platform and marks the current one", () => {
    const onChange = vi.fn();
    render(
      <StepPlatform
        value="vanilla"
        name=""
        onChange={onChange}
        onNameChange={() => {}}
      />,
    );

    expect(screen.getByRole("button", { name: /^Vanilla/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(screen.getByRole("button", { name: /^Fabric/ }));
    expect(onChange).toHaveBeenCalledWith("fabric");
  });
});
