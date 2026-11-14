import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, onTestFinished, vi } from "vitest";

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
          positionKey="search"
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

const manyItems = Array.from({ length: 30 }, (_, index) =>
  modpackListItem({ id: `m${index}`, name: `Many ${index}` }),
);

async function renderManyAt(positionKey: string) {
  const router = createRouter({
    routeTree: createRootRoute({
      component: () => (
        <ModpackList
          positionKey={positionKey}
          items={manyItems}
          hasMore={false}
          loadingMore={false}
          loadMoreFailed={false}
          onLoadMore={vi.fn()}
        />
      ),
    }),
    history: createMemoryHistory({ initialEntries: ["/discover"] }),
  });
  const view = render(<RouterProvider router={router} />);
  // Any row: a restored list does not start at the first one.
  await screen.findAllByRole("link", { name: /Many \d+/ });
  const viewport = view.container.querySelector<HTMLElement>(
    '[data-slot="scroll-area-viewport"]',
  );
  if (viewport === null) throw new Error("no scroll viewport");
  return { viewport, unmount: view.unmount };
}

describe("ModpackList", () => {
  beforeEach(() => {
    mockListLayout();
    // jsdom has no element scrolling; record where the list asks to scroll.
    Object.defineProperty(HTMLElement.prototype, "scrollTo", {
      configurable: true,
      writable: true,
      value(this: HTMLElement, options?: ScrollToOptions) {
        if (options?.top !== undefined) this.scrollTop = options.top;
      },
    });
    onTestFinished(() => {
      Reflect.deleteProperty(HTMLElement.prototype, "scrollTo");
    });
  });

  it("comes back to where the same search was left", async () => {
    const first = await renderManyAt("search:magic");
    first.viewport.scrollTop = 900;
    fireEvent.scroll(first.viewport);
    first.unmount();

    const again = await renderManyAt("search:magic");
    expect(again.viewport.scrollTop).toBe(900);
    again.unmount();

    const other = await renderManyAt("search:tech");
    expect(other.viewport.scrollTop).toBe(0);
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
