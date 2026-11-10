import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, onTestFinished, vi } from "vitest";

import { DiscoverFilters } from "@/features/discover/components/discover-filters";
import { DEFAULT_SEARCH_FILTERS } from "@/features/discover/lib/discover-search";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";
import type { ModpackSource } from "@/lib/generated/ModpackSource";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

function renderFilters(
  filters: ModpackSearchFilters = DEFAULT_SEARCH_FILTERS,
  onChange = vi.fn(),
  sources: readonly ModpackSource[] = ["modrinth", "curseforge"],
) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <DiscoverFilters
        filters={filters}
        sources={sources}
        onChange={onChange}
      />
    </QueryClientProvider>,
  );
  return onChange;
}

describe("DiscoverFilters", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation((command: string) =>
      command === "list_versions"
        ? Promise.resolve([
            { id: "1.21.10", versionType: "release", installed: false },
            { id: "25w45a", versionType: "snapshot", installed: false },
            { id: "1.20.1", versionType: "release", installed: true },
          ])
        : Promise.resolve(undefined),
    );
  });

  it("starts with no narrowing", () => {
    renderFilters();

    expect(screen.getByRole("combobox", { name: "Source" })).toHaveTextContent(
      "Any source",
    );
    expect(screen.getByRole("combobox", { name: "Loader" })).toHaveTextContent(
      "Any loader",
    );
  });

  it("leaves out the Source filter when only Modrinth can be searched", () => {
    renderFilters(DEFAULT_SEARCH_FILTERS, vi.fn(), ["modrinth"]);

    expect(
      screen.queryByRole("combobox", { name: "Source" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/CurseForge/)).not.toBeInTheDocument();
  });

  it("toggles a single category", () => {
    const onChange = renderFilters();
    const categories = screen.getByRole("list", { name: "Category" });

    fireEvent.click(within(categories).getByRole("button", { name: "Magic" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      category: "magic",
    });
  });

  it("clears a category when it is pressed again", () => {
    const onChange = renderFilters({
      ...DEFAULT_SEARCH_FILTERS,
      category: "magic",
    });
    const magic = screen.getByRole("button", { name: "Magic" });
    expect(magic).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(magic);

    expect(onChange).toHaveBeenCalledExactlyOnceWith(DEFAULT_SEARCH_FILTERS);
  });

  it("picks a loader", async () => {
    const onChange = renderFilters();

    fireEvent.click(screen.getByRole("combobox", { name: "Loader" }));
    fireEvent.click(await screen.findByRole("option", { name: "NeoForge" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      loader: "neoforge",
    });
  });

  it("offers only release game versions", async () => {
    // The version list is virtualized; give its viewport a size as jsdom has no layout.
    const height = vi
      .spyOn(HTMLElement.prototype, "offsetHeight", "get")
      .mockImplementation(function (this: HTMLElement) {
        return this.dataset.slot === "combobox-viewport" ? 256 : 0;
      });
    const width = vi
      .spyOn(HTMLElement.prototype, "offsetWidth", "get")
      .mockReturnValue(320);
    onTestFinished(() => {
      height.mockRestore();
      width.mockRestore();
    });
    const onChange = renderFilters();
    const version = screen.getByRole("combobox", { name: "Game version" });

    await vi.waitFor(() => expect(version).toBeEnabled());
    fireEvent.focus(version);
    expect(screen.getByRole("option", { name: "1.21.10" })).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "25w45a" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("option", { name: "1.20.1" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      gameVersion: "1.20.1",
    });
  });

  it("hides Clear filters while nothing narrows the search", () => {
    renderFilters({ ...DEFAULT_SEARCH_FILTERS, sort: "updated" });

    expect(
      screen.queryByRole("button", { name: "Clear filters" }),
    ).not.toBeInTheDocument();
  });

  it("clears every filter but keeps the sort order", () => {
    const onChange = renderFilters({
      gameVersion: "1.20.1",
      loader: "fabric",
      category: "quests",
      source: "modrinth",
      sort: "relevance",
    });

    fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      sort: "relevance",
    });
  });
});
