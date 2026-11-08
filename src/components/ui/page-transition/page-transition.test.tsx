import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { PageTransition } from "@/components/ui/page-transition";

function Page({ title }: { title: string }) {
  const [note, setNote] = useState("");
  return (
    <>
      <h1>{title}</h1>
      <label>
        Page note
        <input value={note} onChange={(event) => setNote(event.target.value)} />
      </label>
    </>
  );
}

function NestedPages({ animationKey }: { animationKey: string }) {
  const [files, setFiles] = useState(false);
  return (
    <PageTransition animationKey={animationKey}>
      <h1>Library page</h1>
      <button type="button" onClick={() => setFiles((current) => !current)}>
        Switch tab
      </button>
      {!files && (
        <PageTransition>
          <p>Content tab</p>
        </PageTransition>
      )}
      {files && (
        <PageTransition>
          <p>Files tab</p>
        </PageTransition>
      )}
    </PageTransition>
  );
}

function transitionFor(text: string) {
  const element = screen
    .getByText(text)
    .closest("[data-slot='page-transition']");
  if (!(element instanceof HTMLElement)) throw new Error("Missing transition");
  return element;
}

function finishAnimation(element: HTMLElement, name = "page-enter") {
  // React falls back to WebKit events when jsdom does not expose AnimationEvent.
  const type =
    "AnimationEvent" in window ? "animationend" : "webkitAnimationEnd";
  const event = new Event(type, { bubbles: true });
  Object.defineProperty(event, "animationName", { value: name });
  fireEvent(element, event);
}

describe("PageTransition", () => {
  it("coordinates nested entries without stacking or delaying the initial tab animation", () => {
    const { rerender } = render(<NestedPages animationKey="classic" />);
    const page = transitionFor("Library page");
    expect(page.className).toContain("motion-safe:animate-page-enter");
    expect(transitionFor("Content tab").className).not.toContain(
      "animate-page-enter",
    );

    finishAnimation(transitionFor("Content tab"));
    fireEvent.click(screen.getByRole("button", { name: "Switch tab" }));
    expect(transitionFor("Files tab").className).not.toContain(
      "animate-page-enter",
    );

    finishAnimation(page);
    expect(transitionFor("Files tab").className).not.toContain(
      "animate-page-enter",
    );
    fireEvent.click(screen.getByRole("button", { name: "Switch tab" }));
    expect(transitionFor("Content tab").className).toContain(
      "motion-safe:animate-page-enter",
    );

    rerender(<NestedPages animationKey="next" />);
    expect(page.className).toContain(
      "motion-safe:animate-page-enter-alternate",
    );
    expect(transitionFor("Content tab").className).not.toContain(
      "animate-page-enter",
    );
    finishAnimation(page, "page-enter-alternate");
    fireEvent.click(screen.getByRole("button", { name: "Switch tab" }));
    expect(transitionFor("Files tab").className).toContain(
      "motion-safe:animate-page-enter",
    );
  });

  it("preserves page input when the current route rerenders", () => {
    const { rerender } = render(
      <PageTransition>
        <Page title="Library" />
      </PageTransition>,
    );
    const input = screen.getByRole("textbox", { name: "Page note" });
    input.focus();
    fireEvent.change(input, { target: { value: "Current page state" } });
    rerender(
      <PageTransition>
        <Page title="Updated library" />
      </PageTransition>,
    );
    expect(
      screen.getByRole("heading", { name: "Updated library" }),
    ).toBeVisible();
    expect(input).toHaveValue("Current page state");
    expect(input).toHaveFocus();
  });

  it("preserves input and focus across animation key changes", () => {
    const { rerender } = render(
      <PageTransition animationKey="classic">
        <Page title="Classic" />
      </PageTransition>,
    );
    const input = screen.getByRole("textbox", { name: "Page note" });
    input.focus();
    fireEvent.change(input, { target: { value: "Keep this input" } });
    for (const { key, direction } of [
      { key: "next", direction: "left" },
      { key: "classic", direction: "right" },
      { key: "classic", direction: "right" },
    ] as const) {
      rerender(
        <PageTransition animationKey={key} direction={direction}>
          <Page title={key} />
        </PageTransition>,
      );
      expect(screen.getByRole("textbox", { name: "Page note" })).toBe(input);
      expect(input).toHaveValue("Keep this input");
      expect(input).toHaveFocus();
      expect(transitionFor(key)).toHaveAttribute("data-direction", direction);
    }
  });

  it("renders the next page without keeping the previous content", () => {
    const { rerender } = render(
      <PageTransition>
        <Page title="Library" />
      </PageTransition>,
    );
    fireEvent.change(screen.getByRole("textbox", { name: "Page note" }), {
      target: { value: "Previous page state" },
    });
    rerender(
      <PageTransition>
        <h1>Downloads</h1>
      </PageTransition>,
    );
    expect(screen.getByRole("heading", { name: "Downloads" })).toBeVisible();
    expect(screen.queryByRole("heading", { name: "Library" })).toBeNull();
    expect(screen.queryByRole("textbox", { name: "Page note" })).toBeNull();
  });
});
