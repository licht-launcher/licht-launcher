import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Reveal } from "@/components/ui/reveal";

function wrapper(container: HTMLElement) {
  return container.querySelector<HTMLElement>("[data-slot='reveal']");
}

describe("Reveal", () => {
  it("renders nothing while it has always been closed", () => {
    const { container } = render(
      <Reveal open={false}>
        <p>Hidden</p>
      </Reveal>,
    );

    expect(wrapper(container)).toBeNull();
    expect(screen.queryByText("Hidden")).toBeNull();
  });

  it("mounts the content collapsed and grows it on the next frame", async () => {
    const { container } = render(
      <Reveal open>
        <p>Shown</p>
      </Reveal>,
    );

    expect(screen.getByText("Shown")).toBeInTheDocument();
    await waitFor(() =>
      expect(wrapper(container)).toHaveAttribute("data-state", "open"),
    );
    expect(wrapper(container)).not.toHaveAttribute("inert");
    expect(wrapper(container)).toHaveClass("grid-rows-[1fr]");
  });

  it("shrinks, locks the content and unmounts it once the transition is over", async () => {
    const { container, rerender } = render(
      <Reveal open>
        <button type="button">Inside</button>
      </Reveal>,
    );
    await waitFor(() =>
      expect(wrapper(container)).toHaveAttribute("data-state", "open"),
    );

    rerender(
      <Reveal open={false}>
        <button type="button">Inside</button>
      </Reveal>,
    );
    await waitFor(() =>
      expect(wrapper(container)).toHaveAttribute("data-state", "closed"),
    );
    expect(wrapper(container)).toHaveAttribute("inert");
    expect(wrapper(container)).toHaveClass("grid-rows-[0fr]");
    await waitFor(() => expect(screen.queryByText("Inside")).toBeNull());
    expect(wrapper(container)).toBeNull();
  });

  it("keeps the content when it opens again before it was removed", async () => {
    const { container, rerender } = render(
      <Reveal open>
        <p>Again</p>
      </Reveal>,
    );
    await waitFor(() =>
      expect(wrapper(container)).toHaveAttribute("data-state", "open"),
    );

    rerender(
      <Reveal open={false}>
        <p>Again</p>
      </Reveal>,
    );
    rerender(
      <Reveal open>
        <p>Again</p>
      </Reveal>,
    );
    await waitFor(() =>
      expect(wrapper(container)).toHaveAttribute("data-state", "open"),
    );
    await new Promise((resolve) => setTimeout(resolve, 400));
    expect(screen.getByText("Again")).toBeInTheDocument();
  });

  it("gives the content room so focus outlines are not clipped", async () => {
    const { container } = render(
      <Reveal open>
        <p>Outlined</p>
      </Reveal>,
    );

    const clip = container.querySelector("[data-slot='reveal-clip']");
    expect(clip).toHaveClass("min-h-0", "overflow-hidden", "-mx-1", "px-1");
    expect(screen.getByText("Outlined").parentElement).toHaveClass("py-1");
  });
});
