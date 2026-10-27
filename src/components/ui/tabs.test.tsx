import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

function SlidingTabs() {
  const [tab, setTab] = useState("content");
  return (
    <Tabs value={tab} onValueChange={setTab}>
      <TabsList variant="sliding">
        <TabsTrigger value="content">Content</TabsTrigger>
        <TabsTrigger value="files">Files</TabsTrigger>
        <TabsTrigger value="console">Console</TabsTrigger>
      </TabsList>
      <TabsContent value="content">Content panel</TabsContent>
      <TabsContent value="files">Files panel</TabsContent>
      <TabsContent value="console">Console panel</TabsContent>
    </Tabs>
  );
}

describe("Tabs", () => {
  const measurements: Record<string, { x: number; width: number }> = {
    Content: { x: 3, width: 96 },
    Files: { x: 99, width: 72 },
    Console: { x: 171, width: 104 },
  };

  beforeEach(() => {
    vi.spyOn(HTMLElement.prototype, "offsetLeft", "get").mockImplementation(
      function (this: HTMLElement) {
        return measurements[this.textContent ?? ""]?.x ?? 0;
      },
    );
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(
      function (this: HTMLElement) {
        return measurements[this.textContent ?? ""]?.width ?? 0;
      },
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("moves the selected highlight with pointer and keyboard navigation", async () => {
    render(<SlidingTabs />);
    const list = screen.getByRole("tablist");
    const indicator = list.querySelector<HTMLElement>(
      "[data-slot='tabs-indicator']",
    );
    if (!indicator) throw new Error("Missing tabs indicator");
    expect(indicator.style.getPropertyValue("--tabs-indicator-x")).toBe("3px");
    expect(indicator.style.getPropertyValue("--tabs-indicator-width")).toBe(
      "96px",
    );
    const files = screen.getByRole("tab", { name: "Files" });
    fireEvent.mouseDown(files);
    expect(screen.getByText("Files panel")).toBeVisible();
    await waitFor(() => {
      expect(indicator.style.getPropertyValue("--tabs-indicator-x")).toBe(
        "99px",
      );
      expect(indicator.style.getPropertyValue("--tabs-indicator-width")).toBe(
        "72px",
      );
    });

    files.focus();
    fireEvent.keyDown(files, { key: "ArrowRight" });
    const consoleTab = screen.getByRole("tab", { name: "Console" });
    await waitFor(() => expect(consoleTab).toHaveFocus());
    expect(consoleTab).toHaveAttribute("aria-selected", "true");
    await waitFor(() => {
      expect(indicator.style.getPropertyValue("--tabs-indicator-x")).toBe(
        "171px",
      );
      expect(indicator.style.getPropertyValue("--tabs-indicator-width")).toBe(
        "104px",
      );
    });
    expect(screen.getByText("Console panel")).toBeVisible();

    fireEvent.keyDown(consoleTab, { key: "Home" });
    const content = screen.getByRole("tab", { name: "Content" });
    await waitFor(() => expect(content).toHaveFocus());
    await waitFor(() => {
      expect(indicator.style.getPropertyValue("--tabs-indicator-x")).toBe(
        "3px",
      );
      expect(indicator.style.getPropertyValue("--tabs-indicator-width")).toBe(
        "96px",
      );
    });
    expect(screen.getByText("Content panel")).toBeVisible();
    fireEvent.keyDown(content, { key: "End" });
    await waitFor(() => expect(consoleTab).toHaveFocus());
    await waitFor(() =>
      expect(indicator.style.getPropertyValue("--tabs-indicator-x")).toBe(
        "171px",
      ),
    );
  });

  it("realigns the indicator after a layout resize and disconnects observers", () => {
    let notifyResize = () => {};
    const disconnect = vi.fn();
    vi.stubGlobal(
      "ResizeObserver",
      class implements ResizeObserver {
        constructor(callback: ResizeObserverCallback) {
          notifyResize = () => callback([], this);
        }
        observe() {}
        unobserve() {}
        disconnect = disconnect;
      },
    );
    const { unmount } = render(<SlidingTabs />);
    const indicator = screen
      .getByRole("tablist")
      .querySelector<HTMLElement>("[data-slot='tabs-indicator']");
    if (!indicator) throw new Error("Missing tabs indicator");
    const previous = measurements.Content;
    measurements.Content = { x: 5, width: 118 };
    try {
      notifyResize();
      expect(indicator.style.getPropertyValue("--tabs-indicator-x")).toBe(
        "5px",
      );
      expect(indicator.style.getPropertyValue("--tabs-indicator-width")).toBe(
        "118px",
      );
      unmount();
      expect(disconnect).toHaveBeenCalledOnce();
    } finally {
      if (previous) measurements.Content = previous;
    }
  });

  it("shows the selected panel", () => {
    render(
      <Tabs defaultValue="one">
        <TabsList>
          <TabsTrigger value="one">Um</TabsTrigger>
          <TabsTrigger value="two">Dois</TabsTrigger>
        </TabsList>
        <TabsContent value="one">Painel um</TabsContent>
        <TabsContent value="two">Painel dois</TabsContent>
      </Tabs>,
    );

    const snapshotTab = screen.getByRole("tab", { name: "Dois" });
    fireEvent.mouseDown(snapshotTab);
    fireEvent.click(snapshotTab);

    expect(screen.getByRole("tab", { name: "Dois" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByText("Painel dois")).toBeVisible();
  });
});
