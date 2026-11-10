import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { modpackSearchPage } from "@/features/discover/fixtures/modpacks";
import { DEFAULT_SEARCH_FILTERS } from "@/features/discover/lib/discover-search";
import {
  invoke,
  mockContextMenuCommands,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";

function mockSearch(respond: () => Promise<unknown>) {
  mockContextMenuCommands((command) =>
    command === "search_modpacks" ? respond() : undefined,
  );
}

function searchCalls() {
  return invoke.mock.calls.filter(([command]) => command === "search_modpacks");
}

describe("Discover page", () => {
  setupInstancesPageTests();

  it("opens from the navigation as a full-width store list", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage()));
    const { router } = await renderAt("/");

    const navigation = screen.getByRole("navigation", {
      name: "Main navigation",
    });
    fireEvent.click(
      within(navigation).getByRole("button", { name: "Discover" }),
    );

    const results = await screen.findByRole("region", { name: "Modpacks" });
    expect(router.state.location.pathname).toBe("/discover");
    expect(
      within(navigation).getByRole("button", { name: "Discover" }),
    ).toHaveAttribute("aria-current", "page");
    expect(
      await within(results).findByRole("link", { name: /All the Mods 9/ }),
    ).toHaveAttribute("href", "/discover/curseforge/381671");
    expect(
      screen.queryByRole("navigation", { name: "Instances" }),
    ).not.toBeInTheDocument();
    expect(searchCalls()).toContainEqual([
      "search_modpacks",
      { query: "", page: 1, filters: DEFAULT_SEARCH_FILTERS },
    ]);
  });

  it("searches once typing pauses and starts again from the first page", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage({ page: 2 })));
    const { router } = await renderAt("/discover?page=2");

    fireEvent.change(
      await screen.findByRole("searchbox", { name: "Search modpacks" }),
      { target: { value: " cobble " } },
    );

    await waitFor(() =>
      expect(router.state.location.search).toEqual({ q: "cobble" }),
    );
    await waitFor(() =>
      expect(searchCalls()).toContainEqual([
        "search_modpacks",
        { query: "cobble", page: 1, filters: DEFAULT_SEARCH_FILTERS },
      ]),
    );
  });

  it("moves to the chosen page and keeps the query", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage()));
    const { router } = await renderAt("/discover?q=atm");

    const pagination = await screen.findByRole("navigation", {
      name: "Pagination",
    });
    fireEvent.click(within(pagination).getByRole("button", { name: "Page 3" }));

    await waitFor(() =>
      expect(router.state.location.search).toEqual({ q: "atm", page: 3 }),
    );
    await waitFor(() =>
      expect(searchCalls()).toContainEqual([
        "search_modpacks",
        { query: "atm", page: 3, filters: DEFAULT_SEARCH_FILTERS },
      ]),
    );
  });

  it("explains when nothing matches", async () => {
    mockSearch(() =>
      Promise.resolve(modpackSearchPage({ items: [], pageCount: 0 })),
    );
    await renderAt("/discover?q=zzzz");

    expect(
      await screen.findByRole("status", { name: "No modpacks found" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("navigation", { name: "Pagination" }),
    ).not.toBeInTheDocument();
  });

  it("shows a friendly error when the search fails", async () => {
    mockSearch(() => Promise.reject("HTTP request failed"));
    await renderAt("/discover");

    const error = await screen.findByRole("status", {
      name: "Could not load modpacks",
    });
    expect(error).toHaveTextContent(
      "The network request failed. Check your connection and try again.",
    );
  });
});
