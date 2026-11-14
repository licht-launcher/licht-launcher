import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Badge } from "@/components/ui/badge";

describe("Badge", () => {
  it("renders its label as plain text", () => {
    render(<Badge>Optimization</Badge>);

    const badge = screen.getByText("Optimization");
    expect(badge.tagName).toBe("SPAN");
    expect(badge).toHaveAttribute("data-slot", "badge");
  });
});
