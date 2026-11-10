import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ModpackList } from "@/features/discover/components/modpack-list";
import { mockListLayout } from "@/features/discover/fixtures/list-layout";
import { modpackListItem } from "@/features/discover/fixtures/modpacks";

type Props = ComponentProps<typeof ModpackList>;

const items = Array.from({ length: 3 }, (_, index) =>
  modpackListItem({ id: `p${index}`, name: `Pack ${index}` }),
);

async function renderList(props: Partial<Props> = {}) {
  const onLoadMore = props.onLoadMore ?? vi.fn();
  const router = createRouter({
    routeTree: createRootRoute({
      component: () => (
        <ModpackList
          items={items}
          hasMore={false}
          loadingMore={false}
          loadMoreFailed={false}
          {...props}
          onLoadMore={onLoadMore}
        />
      ),
    }),
    history: createMemoryHistory({ initialEntries: ["/discover"] }),
  });
  render(<RouterProvider router={router} />);
  await screen.findByRole("link", { name: /Pack 0/ });
  return onLoadMore;
}

describe("ModpackList", () => {
  beforeEach(() => {
    mockListLayout();
  });

  it("lists every result with its position", async () => {
    await renderList();

    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(rows[2]).toHaveAttribute("aria-posinset", "3");
    expect(rows[2]).toHaveAttribute("aria-setsize", "3");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("asks for more once the end is near", async () => {
    // Repeat calls are fine: the page ignores them while a page is loading,
    // which the page test checks through the requests it sends.
    const onLoadMore = await renderList({ hasMore: true });

    expect(onLoadMore).toHaveBeenCalled();
    expect(screen.getAllByRole("listitem")[2]).toHaveAttribute(
      "aria-setsize",
      "-1",
    );
  });

  it("shows progress and does not ask again while a page is loading", async () => {
    const onLoadMore = await renderList({ hasMore: true, loadingMore: true });

    expect(onLoadMore).not.toHaveBeenCalled();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Loading more modpacks…",
    );
  });

  it("waits for Retry after a page failed", async () => {
    const onLoadMore = await renderList({
      hasMore: true,
      loadMoreFailed: true,
    });
    expect(onLoadMore).not.toHaveBeenCalled();

    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("Could not load more modpacks.");
    fireEvent.click(within(alert).getByRole("button", { name: "Retry" }));

    expect(onLoadMore).toHaveBeenCalledOnce();
  });
});
