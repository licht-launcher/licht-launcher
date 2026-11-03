import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { FormSection } from "@/features/new-instance/components/form-section";

describe("FormSection", () => {
  it("shows a small title above a bordered card, like the settings sections", () => {
    render(
      <FormSection title="Platform">
        <p>Content</p>
      </FormSection>,
    );

    const heading = screen.getByRole("heading", { level: 3, name: "Platform" });
    expect(heading).toHaveClass("text-xs", "text-muted-foreground");
    const card = screen.getByRole("region", { name: "Platform" });
    expect(card).toHaveAttribute("data-slot", "card");
    expect(card).toContainElement(screen.getByText("Content"));
    expect(heading.parentElement).toContainElement(card);
  });

  it("puts the hint inside the card, in the muted settings style", () => {
    render(
      <FormSection title="Game version" description="Type to search.">
        <p>Content</p>
      </FormSection>,
    );

    const hint = screen.getByText("Type to search.");
    expect(hint).toHaveClass("text-sm", "text-muted-foreground");
    expect(
      screen.getByRole("region", { name: "Game version" }),
    ).toContainElement(hint);
  });

  it("has no icon well", () => {
    const { container } = render(
      <FormSection title="Name">
        <p>Content</p>
      </FormSection>,
    );

    expect(container.querySelector('[class*="bg-instance"]')).toBeNull();
  });
});
