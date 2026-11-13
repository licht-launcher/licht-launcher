import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { CategoryBadge } from "./category-badge";

describe("CategoryBadge", () => {
  it("names the category next to its icon", () => {
    const { container } = render(<CategoryBadge category="kitchenSink" />);

    const badge = screen.getByText("Kitchen sink");
    expect(badge).toHaveAttribute("data-slot", "badge");
    expect(badge).toHaveAttribute("data-category", "kitchenSink");
    expect(container.querySelector("svg")).not.toBeNull();
  });

  it("uses the same border for every category", () => {
    render(
      <>
        <CategoryBadge category="magic" />
        <CategoryBadge category="technology" />
      </>,
    );

    const magic = screen.getByText("Magic");
    expect(magic.className).toContain("border-border");
    expect(screen.getByText("Technology").className).toBe(magic.className);
  });
});
