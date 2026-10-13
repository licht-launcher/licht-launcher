import { act, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { LogLevelSelection } from "@/features/instances/lib/log-level";
import type { ConsoleLine } from "@/features/instances/model/console-line";
import {
  ConsoleLogLevelFilter,
  ConsoleSection,
} from "@/features/instances/sections/console-section";

const lines: ConsoleLine[] = [
  { id: 1, stream: "stdout", line: "[t/INFO]: hello" },
  { id: 2, stream: "stdout", line: "[t/WARN]: careful" },
  { id: 3, stream: "stdout", line: "[t/ERROR]: boom" },
];

function ConsoleHarness() {
  const [selected, setSelected] = useState<LogLevelSelection>(() => new Set());
  return (
    <>
      <ConsoleLogLevelFilter
        selected={selected}
        onSelectedChange={setSelected}
      />
      <ConsoleSection lines={lines} selected={selected} />
    </>
  );
}

function stubViewport(viewport: HTMLElement, nearBottom: boolean) {
  Object.defineProperty(viewport, "scrollHeight", {
    configurable: true,
    get: () => 1000,
  });
  Object.defineProperty(viewport, "clientHeight", {
    configurable: true,
    get: () => 200,
  });
  Object.defineProperty(viewport, "scrollTop", {
    configurable: true,
    writable: true,
    value: nearBottom ? 800 : 0,
  });
}

describe("ConsoleSection", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn(() => ({ matches: false })),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("shows empty copy with simple decorative flowers", () => {
    render(<ConsoleSection lines={[]} selected={new Set()} />);
    const log = screen.getByLabelText("Game log");
    expect(log).toHaveTextContent("The game log will appear here.");
    const flowers = screen.getByText("❀ ✿ ❁");
    expect(flowers).toHaveAttribute("aria-hidden", "true");
  });

  it("shows a mono console and filters by multi level selection", async () => {
    render(<ConsoleHarness />);

    const log = screen.getByLabelText("Game log");
    expect(log).toBeInTheDocument();
    expect(log.querySelector("[data-slot='scroll-area']")).toHaveClass(
      "border-border/60",
      "bg-black/20",
    );
    expect(log.querySelector("pre")).toHaveClass("font-mono");
    expect(log.querySelector("pre")).toHaveClass("select-text");
    expect(log.querySelector("pre")).toHaveClass("cursor-text");
    expect(log.querySelector("pre")?.className).toMatch(
      /selection:bg-muted-foreground/,
    );
    expect(log).toHaveTextContent("[t/INFO]: hello");
    expect(log).toHaveTextContent("[t/WARN]: careful");
    expect(log).toHaveTextContent("[t/ERROR]: boom");
    expect(screen.getByText("INFO")).toHaveClass("text-info");
    expect(screen.getByText("WARN")).toHaveClass("text-warning");
    expect(screen.getByText("ERROR")).toHaveClass("text-destructive");

    fireEvent.click(screen.getByRole("button", { name: "Log level" }));
    expect(await screen.findByText("Log Level")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Warn" }));
    expect(screen.getByRole("button", { name: "Log level" })).toHaveTextContent(
      "Warn",
    );
    expect(log).not.toHaveTextContent("hello");
    expect(log).toHaveTextContent("careful");
    expect(log).not.toHaveTextContent("boom");

    fireEvent.click(screen.getByRole("button", { name: "Error" }));
    expect(screen.getByRole("button", { name: "Log level" })).toHaveTextContent(
      "Warn + Error",
    );
    expect(log).not.toHaveTextContent("hello");
    expect(log).toHaveTextContent("careful");
    expect(log).toHaveTextContent("boom");
  });

  it("scrolls smoothly to the bottom without snapping the viewport", () => {
    render(<ConsoleSection lines={lines} selected={new Set()} />);
    const log = screen.getByLabelText("Game log");
    const viewport = log.querySelector(
      "[data-slot='scroll-area-viewport']",
    ) as HTMLElement;
    stubViewport(viewport, false);
    const scrollTo = vi.fn();
    Object.defineProperty(viewport, "scrollTo", {
      configurable: true,
      value: scrollTo,
    });

    expect(
      screen.queryByRole("button", { name: "Scroll to bottom" }),
    ).not.toBeInTheDocument();

    act(() => {
      fireEvent.scroll(viewport);
    });

    expect(
      screen.getByRole("button", { name: "Scroll to bottom" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Scroll to bottom" }));
    expect(scrollTo).toHaveBeenCalledWith({ top: 1000, behavior: "smooth" });
    expect(viewport.scrollTop).toBe(0);

    stubViewport(viewport, true);
    fireEvent.scroll(viewport);
    expect(
      screen.queryByRole("button", { name: "Scroll to bottom" }),
    ).not.toBeInTheDocument();
  });

  it("uses immediate scrolling when reduced motion is preferred", () => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn(() => ({ matches: true })),
    );
    render(<ConsoleSection lines={lines} selected={new Set()} />);
    const viewport = screen
      .getByLabelText("Game log")
      .querySelector("[data-slot='scroll-area-viewport']") as HTMLElement;
    stubViewport(viewport, false);
    const scrollTo = vi.fn();
    Object.defineProperty(viewport, "scrollTo", {
      configurable: true,
      value: scrollTo,
    });

    fireEvent.scroll(viewport);
    fireEvent.click(screen.getByRole("button", { name: "Scroll to bottom" }));

    expect(scrollTo).toHaveBeenCalledWith({ top: 1000, behavior: "auto" });
  });

  it("reveals the scrollbar for two seconds after the latest incoming line", () => {
    vi.useFakeTimers();
    const { rerender } = render(
      <ConsoleSection lines={[]} selected={new Set()} />,
    );
    const log = screen.getByLabelText("Game log");
    const scrollbar = () =>
      log.querySelector("[data-slot='scroll-area-scrollbar']");

    expect(scrollbar()).not.toBeInTheDocument();
    rerender(<ConsoleSection lines={lines} selected={new Set()} />);
    expect(scrollbar()).toHaveAttribute("data-state", "visible");

    act(() => vi.advanceTimersByTime(1900));
    rerender(
      <ConsoleSection
        lines={[
          ...lines.slice(1),
          { id: 4, stream: "stdout", line: "[t/INFO]: next line" },
        ]}
        selected={new Set()}
      />,
    );
    act(() => vi.advanceTimersByTime(1900));
    expect(scrollbar()).toHaveAttribute("data-state", "visible");

    act(() => vi.advanceTimersByTime(100));
    expect(scrollbar()).not.toBeInTheDocument();
  });

  it("does not pull a reader down when a new line arrives", () => {
    const { rerender } = render(
      <ConsoleSection lines={lines} selected={new Set()} />,
    );
    const log = screen.getByLabelText("Game log");
    const viewport = log.querySelector(
      "[data-slot='scroll-area-viewport']",
    ) as HTMLElement;
    stubViewport(viewport, false);
    fireEvent.scroll(viewport);

    rerender(
      <ConsoleSection
        lines={[...lines, { id: 4, stream: "stdout", line: "new line" }]}
        selected={new Set()}
      />,
    );

    expect(viewport.scrollTop).toBe(0);
    expect(
      screen.getByRole("button", { name: "Scroll to bottom" }),
    ).toBeInTheDocument();
    expect(
      log.querySelector("[data-slot='scroll-area-scrollbar']"),
    ).toHaveAttribute("data-state", "visible");
  });
});
