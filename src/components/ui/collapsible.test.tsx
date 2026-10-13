import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

describe("Collapsible", () => {
  it("shows and hides content when the trigger is pressed", () => {
    render(
      <Collapsible>
        <CollapsibleTrigger>1.20</CollapsibleTrigger>
        <CollapsibleContent>1.20.1</CollapsibleContent>
      </Collapsible>,
    );

    expect(screen.queryByText("1.20.1")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "1.20" }));
    expect(screen.getByText("1.20.1")).toBeInTheDocument();
  });
});
