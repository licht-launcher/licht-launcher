import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ContentFilterSelect } from "./content-filter-select";

describe("ContentFilterSelect", () => {
  it("shows the current filter", () => {
    render(
      <ContentFilterSelect value="resourcePacks" onValueChange={vi.fn()} />,
    );

    expect(
      screen.getByRole("combobox", { name: "Filter content" }),
    ).toHaveTextContent("Resource Packs");
  });

  it("reports the kind the user picks", async () => {
    const onValueChange = vi.fn();
    render(<ContentFilterSelect value="all" onValueChange={onValueChange} />);

    fireEvent.click(screen.getByRole("combobox", { name: "Filter content" }));
    fireEvent.click(
      await screen.findByRole("option", { name: "Shader Packs" }),
    );

    expect(onValueChange).toHaveBeenCalledWith("shaderPacks");
  });
});
