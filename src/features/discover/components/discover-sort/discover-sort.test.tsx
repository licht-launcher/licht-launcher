import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { DiscoverSort } from "@/features/discover/components/discover-sort";

describe("DiscoverSort", () => {
  it("shows the current order and picks another", async () => {
    const onChange = vi.fn();
    render(<DiscoverSort value="downloads" onChange={onChange} />);

    const trigger = screen.getByRole("combobox", { name: "Sort by" });
    expect(trigger).toHaveTextContent("Downloads");
    fireEvent.click(trigger);
    const options = await screen.findAllByRole("option");
    // Each order is drawn with its own icon.
    for (const option of options) {
      expect(option.querySelector("svg")).not.toBeNull();
    }
    fireEvent.click(screen.getByRole("option", { name: "Relevance" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith("relevance");
  });
});
