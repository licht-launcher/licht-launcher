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
    const loaders = screen.getByRole("list", { name: "Loader" });
    for (const name of ["Fabric", "Forge", "NeoForge", "Quilt"]) {
      expect(within(loaders).getByRole("button", { name })).toHaveAttribute(
        "aria-pressed",
        "false",
      );
    }
  });

  it("leaves out the Source filter when only Modrinth can be searched", () => {
    renderFilters(DEFAULT_SEARCH_FILTERS, vi.fn(), ["modrinth"]);

    expect(
      screen.queryByRole("combobox", { name: "Source" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/CurseForge/)).not.toBeInTheDocument();
  });

  it("collapses and expands a filter group", () => {
    renderFilters();
    const toggle = screen.getByRole("button", { name: "Loader" });
    expect(toggle).toHaveAttribute("aria-expanded", "true");

    fireEvent.click(toggle);

    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(
      screen.queryByRole("list", { name: "Loader" }),
    ).not.toBeInTheDocument();

    fireEvent.click(toggle);

    expect(screen.getByRole("list", { name: "Loader" })).toBeInTheDocument();
  });

  it("adds a category to the ones already chosen", () => {
    const onChange = renderFilters({
      ...DEFAULT_SEARCH_FILTERS,
      categories: ["technology"],
    });
    const categories = screen.getByRole("list", { name: "Category" });

    fireEvent.click(within(categories).getByRole("button", { name: "Magic" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      categories: ["magic", "technology"],
    });
  });

  it("removes a category when it is pressed again", () => {
    const onChange = renderFilters({
      ...DEFAULT_SEARCH_FILTERS,
      categories: ["magic", "quests"],
    });
    const magic = screen.getByRole("button", { name: "Magic" });
    expect(magic).toHaveAttribute("aria-pressed", "true");
    // Chosen categories share the neutral background and a border.
    expect(magic.className).toBe(
      screen.getByRole("button", { name: "Quests" }).className,
    );
    expect(magic.className).toContain("aria-pressed:border-border");
    expect(magic.className).toContain("aria-pressed:bg-secondary");

    fireEvent.click(magic);

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      categories: ["quests"],
    });
  });

  it("disables categories CurseForge lacks in a CurseForge-only search", () => {
    renderFilters({ ...DEFAULT_SEARCH_FILTERS, source: "curseforge" });

    expect(screen.getByRole("button", { name: "Optimization" })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Optimization" }),
    ).toHaveAttribute("title", "CurseForge has no Optimization category");
    expect(screen.getByRole("button", { name: "Magic" })).toBeEnabled();
  });

  it("drops categories CurseForge lacks when picking CurseForge", async () => {
    const onChange = renderFilters({
      ...DEFAULT_SEARCH_FILTERS,
      categories: ["magic", "optimization"],
    });

    fireEvent.click(screen.getByRole("combobox", { name: "Source" }));
    fireEvent.click(await screen.findByRole("option", { name: "CurseForge" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      source: "curseforge",
      categories: ["magic"],
    });
  });

  it("picks a loader", () => {
    const onChange = renderFilters();

    fireEvent.click(screen.getByRole("button", { name: "NeoForge" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      loaders: ["neoforge"],
    });
  });

  it("adds a loader to the ones already chosen", () => {
    const onChange = renderFilters({
      ...DEFAULT_SEARCH_FILTERS,
      loaders: ["fabric"],
    });

    fireEvent.click(screen.getByRole("button", { name: "Quilt" }));

    expect(onChange).toHaveBeenCalledExactlyOnceWith({
      ...DEFAULT_SEARCH_FILTERS,
      loaders: ["fabric", "quilt"],
    });
  });

  it("clears a loader when it is pressed again", () => {
    const onChange = renderFilters({
      ...DEFAULT_SEARCH_FILTERS,
      loaders: ["neoforge"],
    });
    const neoforge = screen.getByRole("button", { name: "NeoForge" });
    expect(neoforge).toHaveAttribute("aria-pressed", "true");
    expect(neoforge.className).toContain("border-accent-orange/25");
    expect(neoforge.className).toContain("hover:bg-accent-orange/15");
    expect(neoforge.className).not.toContain("hover:bg-accent-orange/25");
    expect(neoforge.className).not.toBe(
      screen.getByRole("button", { name: "Fabric" }).className,
    );

    fireEvent.click(neoforge);

    expect(onChange).toHaveBeenCalledExactlyOnceWith(DEFAULT_SEARCH_FILTERS);
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
});
