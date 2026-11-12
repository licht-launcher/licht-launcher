import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ActiveFilters } from "@/features/discover/components/active-filters";
import { DEFAULT_SEARCH_FILTERS } from "@/features/discover/lib/discover-search";

describe("ActiveFilters", () => {
  it("shows nothing while no filter narrows the search", () => {
    render(
      <ActiveFilters
        filters={{ ...DEFAULT_SEARCH_FILTERS, sort: "updated" }}
        onChange={vi.fn()}
      />,
    );

    expect(
      screen.queryByRole("list", { name: "Active filters" }),
    ).not.toBeInTheDocument();
  });

  it("lists every active filter", () => {
    render(
      <ActiveFilters
        filters={{
          ...DEFAULT_SEARCH_FILTERS,
          source: "curseforge",
          gameVersion: "1.20.1",
          loader: "neoforge",
          categories: ["kitchenSink", "magic"],
        }}
        onChange={vi.fn()}
      />,
    );

    const list = screen.getByRole("list", { name: "Active filters" });
    expect(
      within(list)
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["CurseForge", "1.20.1", "NeoForge", "Kitchen sink", "Magic"]);
    // Loader and categories keep their own colors.
    expect(within(list).getByText("Kitchen sink")).toHaveAttribute(
      "data-category",
      "kitchenSink",
    );
    expect(within(list).getByText("NeoForge")).toHaveAttribute(
      "data-loader",
      "neoforge",
    );
  });

  it("removes only the pressed filter and keeps the sort order", () => {
    const onChange = vi.fn();
    const filters = {
      ...DEFAULT_SEARCH_FILTERS,
      sort: "relevance" as const,
      loader: "fabric" as const,
      categories: ["magic" as const, "quests" as const],
    };
    render(<ActiveFilters filters={filters} onChange={onChange} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Remove Magic filter" }),
    );

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...filters,
      categories: ["quests"],
    });
  });
});
