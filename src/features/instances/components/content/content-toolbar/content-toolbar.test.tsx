import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ContentToolbar } from "./content-toolbar";

function renderToolbar(refreshing = false) {
  const handlers = {
    onSearchChange: vi.fn(),
    onFilterChange: vi.fn(),
    onRefresh: vi.fn(),
  };
  render(
    <ContentToolbar
      search="sod"
      filter="mods"
      refreshing={refreshing}
      {...handlers}
    />,
  );
  return handlers;
}

describe("ContentToolbar", () => {
  it("shows the current search and filter and reports typing", () => {
    const { onSearchChange } = renderToolbar();

    const search = screen.getByRole("textbox", { name: "Search content" });
    expect(search).toHaveValue("sod");
    expect(
      screen.getByRole("combobox", { name: "Filter content" }),
    ).toHaveTextContent("Mods");
    fireEvent.change(search, { target: { value: "sodium" } });
    expect(onSearchChange).toHaveBeenCalledWith("sodium");
  });

  it("refreshes on request", () => {
    const { onRefresh } = renderToolbar();
    fireEvent.click(screen.getByRole("button", { name: "Refresh content" }));
    expect(onRefresh).toHaveBeenCalledOnce();
  });

  it("disables refresh while one is running", () => {
    renderToolbar(true);
    expect(
      screen.getByRole("button", { name: "Refresh content" }),
    ).toBeDisabled();
  });
});
