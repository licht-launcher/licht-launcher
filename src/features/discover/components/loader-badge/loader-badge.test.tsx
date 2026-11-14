import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { LoaderBadge } from "./loader-badge";

describe("LoaderBadge", () => {
  it("names the loader next to its logo", () => {
    const { container } = render(<LoaderBadge loader="neoforge" />);

    const badge = screen.getByText("NeoForge");
    expect(badge).toHaveAttribute("data-slot", "badge");
    expect(badge).toHaveAttribute("data-loader", "neoforge");
    expect(container.querySelector("img")?.getAttribute("src")).toContain(
      "neoforge",
    );
  });

  it("gives each loader its own color", () => {
    render(
      <>
        <LoaderBadge loader="fabric" />
        <LoaderBadge loader="forge" />
      </>,
    );

    expect(screen.getByText("Fabric").className).not.toBe(
      screen.getByText("Forge").className,
    );
  });
});
