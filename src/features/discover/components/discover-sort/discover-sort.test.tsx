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
    fireEvent.click(await screen.findByRole("option", { name: "Relevance" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith("relevance");
  });
});
