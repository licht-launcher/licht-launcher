import { render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it } from "vitest";

import { ScrollArea } from "@/components/ui/scroll-area";

describe("ScrollArea", () => {
  it("keeps content inside the scrollable viewport", () => {
    render(
      <ScrollArea>
        <button type="button">Rolar</button>
      </ScrollArea>,
    );

    const button = screen.getByRole("button", { name: "Rolar" });

    button.focus();

    expect(button.closest("[data-slot=scroll-area-viewport]")).not.toBeNull();
    expect(button).toHaveFocus();
  });

  it("exposes the actual scroll viewport through its ref", () => {
    const viewport = createRef<HTMLDivElement>();
    const { unmount } = render(
      <ScrollArea viewportRef={viewport}>
        <button type="button">Content</button>
      </ScrollArea>,
    );
    expect(viewport.current).toBe(
      screen
        .getByRole("button", { name: "Content" })
        .closest("[data-slot=scroll-area-viewport]"),
    );
    expect(viewport.current).toHaveAttribute(
      "data-slot",
      "scroll-area-viewport",
    );
    unmount();
    expect(viewport.current).toBeNull();
  });
});
