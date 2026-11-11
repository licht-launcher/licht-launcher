import { focusManager } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, onTestFinished, vi } from "vitest";

import { mockListLayout } from "@/features/discover/fixtures/list-layout";
import {
  modpackListItem,
  modpackSearchPage,
} from "@/features/discover/fixtures/modpacks";
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

/** Page `n` of three, each with its own two packs. */
function pageOf(args: unknown) {
  const page =
    args !== null && typeof args === "object" && "page" in args
      ? Number(args.page)
      : 1;
  return modpackSearchPage({
    page,
    pageCount: 3,
    items: [
      modpackListItem({ id: `${page}-a`, name: `Pack ${page}A` }),
      modpackListItem({ id: `${page}-b`, name: `Pack ${page}B` }),
    ],
  });
}

function requestedPages() {
  return searchCalls().map(([, args]) =>
    args !== null && typeof args === "object" && "page" in args
      ? args.page
      : undefined,
  );
}

function searchCalls() {
  return invoke.mock.calls.filter(([command]) => command === "search_modpacks");
}

describe("Discover page", () => {
  setupInstancesPageTests();
  beforeEach(() => {
    mockListLayout();
  });

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

  it("searches once typing pauses", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage({ pageCount: 1 })));
    const { router } = await renderAt("/discover");

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

  it("loads the following pages as the end of the list comes into view", async () => {
    mockContextMenuCommands((command, args) =>
      command === "search_modpacks" ? Promise.resolve(pageOf(args)) : undefined,
    );
    await renderAt("/discover?q=atm");

    const results = await screen.findByRole("region", { name: "Modpacks" });
    expect(
      await within(results).findByRole("link", { name: /Pack 3B/ }),
    ).toBeInTheDocument();
    expect(requestedPages()).toEqual([1, 2, 3]);
    expect(searchCalls()).toContainEqual([
      "search_modpacks",
      { query: "atm", page: 3, filters: DEFAULT_SEARCH_FILTERS },
    ]);
    expect(within(results).queryByRole("status")).not.toBeInTheDocument();
  });

  it("offers a retry when the next page fails", async () => {
    let failSecondPage = true;
    mockContextMenuCommands((command, args) => {
      if (command !== "search_modpacks") return undefined;
      const page = pageOf(args);
      if (page.page === 2 && failSecondPage) {
        failSecondPage = false;
        return Promise.reject("HTTP request failed");
      }
      return Promise.resolve({ ...page, pageCount: 2 });
    });
    await renderAt("/discover");

    const results = await screen.findByRole("region", { name: "Modpacks" });
    const alert = await within(results).findByRole("alert");
    expect(alert).toHaveTextContent("Could not load more modpacks.");
    expect(
      within(results).getByRole("link", { name: /Pack 1A/ }),
    ).toBeVisible();
    expect(requestedPages()).toEqual([1, 2]);

    fireEvent.click(within(alert).getByRole("button", { name: "Retry" }));

    expect(
      await within(results).findByRole("link", { name: /Pack 2B/ }),
    ).toBeInTheDocument();
    expect(requestedPages()).toEqual([1, 2, 2]);
  });

  it("searches with the filters in the URL", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage()));
    await renderAt(
      "/discover?q=atm&loader=fabric&category=magic&source=curseforge&sort=updated",
    );

    expect(
      await screen.findByRole("button", { name: "Magic" }),
    ).toHaveAttribute("aria-pressed", "true");
    await waitFor(() =>
      expect(searchCalls()).toContainEqual([
        "search_modpacks",
        {
          query: "atm",
          page: 1,
          filters: {
            gameVersion: null,
            loader: "fabric",
            category: "magic",
            source: "curseforge",
            sort: "updated",
          },
        },
      ]),
    );
  });

  it("applies a filter from the sidebar and starts a new search", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage({ pageCount: 1 })));
    const { router } = await renderAt("/discover?q=atm");

    const filters = await screen.findByRole("complementary", {
      name: "Filters",
    });
    fireEvent.click(within(filters).getByRole("button", { name: "Quests" }));

    await waitFor(() =>
      expect(router.state.location.search).toEqual({
        q: "atm",
        category: "quests",
      }),
    );
    await waitFor(() =>
      expect(searchCalls()).toContainEqual([
        "search_modpacks",
        {
          query: "atm",
          page: 1,
          filters: { ...DEFAULT_SEARCH_FILTERS, category: "quests" },
        },
      ]),
    );
  });

  it("sorts from the control next to the search box", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage()));
    const { router } = await renderAt("/discover?q=atm&category=magic");

    fireEvent.click(await screen.findByRole("combobox", { name: "Sort by" }));
    fireEvent.click(
      await screen.findByRole("option", { name: "Recently updated" }),
    );

    await waitFor(() =>
      expect(router.state.location.search).toEqual({
        q: "atm",
        category: "magic",
        sort: "updated",
      }),
    );
    expect(
      within(
        screen.getByRole("complementary", { name: "Filters" }),
      ).queryByRole("combobox", { name: "Sort by" }),
    ).not.toBeInTheDocument();
  });

  it("keeps the filters when the search text changes", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage()));
    const { router } = await renderAt("/discover?loader=quilt");

    fireEvent.change(
      await screen.findByRole("searchbox", { name: "Search modpacks" }),
      { target: { value: "sky" } },
    );

    await waitFor(() =>
      expect(router.state.location.search).toEqual({
        q: "sky",
        loader: "quilt",
      }),
    );
  });

  it("does not show the previous results while a new search loads", async () => {
    mockContextMenuCommands((command, args) => {
      if (command !== "search_modpacks") return undefined;
      const filters =
        args !== null && typeof args === "object" && "filters" in args
          ? (args.filters as { category: string | null })
          : { category: null };
      // The filtered search never answers, so it stays loading.
      return filters.category === null
        ? Promise.resolve(modpackSearchPage({ pageCount: 1 }))
        : new Promise(() => {});
    });
    await renderAt("/discover");

    const results = await screen.findByRole("region", { name: "Modpacks" });
    await within(results).findByRole("link", { name: /Fabulously Optimized/ });
    fireEvent.click(screen.getByRole("button", { name: "Magic" }));

    await waitFor(() =>
      expect(
        within(results).queryByRole("link", { name: /Fabulously Optimized/ }),
      ).not.toBeInTheDocument(),
    );
    expect(results).toHaveAttribute("aria-busy", "true");
  });

  it("does not reload every loaded page when the window regains focus", async () => {
    mockSearch(() => Promise.resolve(modpackSearchPage({ pageCount: 1 })));
    await renderAt("/discover");
    await screen.findByRole("link", { name: /Fabulously Optimized/ });
    const before = searchCalls().length;
    // Only Date is faked: the results are now older than their fresh window.
    vi.useFakeTimers({ toFake: ["Date"] });
    onTestFinished(() => {
      vi.useRealTimers();
    });
    vi.setSystemTime(Date.now() + 60 * 60_000);

    act(() => {
      focusManager.setFocused(false);
      focusManager.setFocused(true);
    });

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(searchCalls()).toHaveLength(before);
  });

  it("explains when nothing matches", async () => {
    mockSearch(() =>
      Promise.resolve(modpackSearchPage({ items: [], pageCount: 0 })),
    );
    await renderAt("/discover?q=zzzz");

    expect(
      await screen.findByRole("status", { name: "No modpacks found" }),
    ).toBeInTheDocument();
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
