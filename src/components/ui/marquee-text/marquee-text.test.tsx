import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { MarqueeText } from "@/components/ui/marquee-text";

describe("MarqueeText", () => {
  it("shows its whole text on a single line", () => {
    render(<MarqueeText>15.0.0-beta.3+fabric.1.21.1</MarqueeText>);

    const text = screen.getByText("15.0.0-beta.3+fabric.1.21.1");
    expect(text).toBeVisible();
    expect(text.parentElement).toHaveClass("whitespace-nowrap");
  });

  it("only scrolls when the user allows motion", () => {
    render(<MarqueeText>Long version name</MarqueeText>);

    expect(screen.getByText("Long version name")).toHaveClass(
      "motion-safe:animate-marquee",
    );
  });
});
