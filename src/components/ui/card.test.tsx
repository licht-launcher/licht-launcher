import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

describe("Card", () => {
  it("renders the supporting title outside the container", () => {
    render(
      <Card title="Theme">
        <span>Color theme</span>
      </Card>,
    );

    const heading = screen.getByRole("heading", { name: "Theme" });
    const region = screen.getByRole("region", { name: "Theme" });
    expect(region).not.toContainElement(heading);
    expect(region).toHaveTextContent("Color theme");
    expect(
      heading.compareDocumentPosition(region) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("groups content in a labelled region and preserves child interactions", () => {
    const onClick = vi.fn();
    render(
      <Card aria-labelledby="appearance-heading">
        <h3 id="appearance-heading">Appearance</h3>
        <Button onClick={onClick}>Change theme</Button>
      </Card>,
    );

    const region = screen.getByRole("region", { name: "Appearance" });
    fireEvent.click(
      within(region).getByRole("button", { name: "Change theme" }),
    );

    expect(onClick).toHaveBeenCalledOnce();
  });
});
