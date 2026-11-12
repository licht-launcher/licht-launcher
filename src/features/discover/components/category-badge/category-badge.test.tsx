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

  it("gives each category its own color", () => {
    render(
      <>
        <CategoryBadge category="magic" />
        <CategoryBadge category="technology" />
      </>,
    );

    expect(screen.getByText("Magic").className).not.toBe(
      screen.getByText("Technology").className,
    );
  });
});
