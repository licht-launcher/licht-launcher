import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";

describe("Resizable", () => {
  it("renders a horizontal group with a handle", () => {
    render(
      <div className="h-40 w-96">
        <ResizablePanelGroup orientation="horizontal" id="test-group">
          <ResizablePanel id="left" defaultSize={120} minSize={80}>
            Left
          </ResizablePanel>
          <ResizableHandle />
          <ResizablePanel id="right" minSize={80}>
            Right
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>,
    );

    expect(screen.getByText("Left")).toBeInTheDocument();
    expect(screen.getByText("Right")).toBeInTheDocument();
    expect(
      document.querySelector('[data-slot="resizable-handle"]'),
    ).toBeInTheDocument();
  });
});
