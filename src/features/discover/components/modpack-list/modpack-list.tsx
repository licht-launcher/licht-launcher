import {
  defaultRangeExtractor,
  type Range,
  useVirtualizer,
  type VirtualItem,
} from "@tanstack/react-virtual";
import {
  type FocusEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { ModpackRow } from "@/features/discover/components/modpack-row";
import { modpackKey } from "@/features/discover/lib/merge-pages";
import type { ModpackListItem } from "@/lib/generated/ModpackListItem";

import { modpackListStyles } from "./modpack-list.styles";

/** Initial row height guess; real heights are measured once rows render. */
const ROW_ESTIMATE_PX = 72;
/** Start loading the next page when this many rows are left below the window. */
const LOAD_AHEAD_ROWS = 5;
const FOOTER_KEY = "load-more";

type ListPosition = {
  key: string;
  itemCount: number;
  offset: number;
  measurements: VirtualItem[];
};

/**
 * Where the last list was left. Opening a pack unmounts the list, so coming
 * back to the same search restores this instead of starting at the top.
 */
let lastPosition: ListPosition | null = null;

/**
 * Virtualized, endless list of search results: only the rows in view (plus a
 * few) are in the DOM, and the next page is requested before the end shows.
 * `positionKey` names the search, so its scroll position can be restored.
 */
export function ModpackList({
  positionKey,
  items,
  hasMore,
  loadingMore,
  loadMoreFailed,
  onLoadMore,
}: {
  positionKey: string;
  items: readonly ModpackListItem[];
  hasMore: boolean;
  loadingMore: boolean;
  loadMoreFailed: boolean;
  onLoadMore: () => void;
}) {
  const viewportRef = useRef<HTMLDivElement>(null);
  // Only restore when the rows it was measured against are loaded again.
  const [restored] = useState(() =>
    lastPosition?.key === positionKey && items.length >= lastPosition.itemCount
      ? lastPosition
      : null,
  );
  const itemCount = useRef(items.length);
  const [focusedKey, setFocusedKey] = useState<string | null>(null);
  const keys = useMemo(() => items.map(modpackKey), [items]);
  const focusedIndex = focusedKey === null ? -1 : keys.indexOf(focusedKey);
  const showFooter = hasMore || loadMoreFailed;

  // Keep the focused row mounted while it scrolls out, so keyboard focus survives.
  const rangeExtractor = useCallback(
    (range: Range) => {
      const indexes = new Set(defaultRangeExtractor(range));
      if (focusedIndex >= 0) indexes.add(focusedIndex);
      return [...indexes].sort((left, right) => left - right);
    },
    [focusedIndex],
  );
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLLIElement>({
    count: items.length + (showFooter ? 1 : 0),
    getScrollElement: () => viewportRef.current,
    estimateSize: () => ROW_ESTIMATE_PX,
    getItemKey: (index) => keys[index] ?? FOOTER_KEY,
    rangeExtractor,
    overscan: 4,
    initialOffset: restored?.offset ?? 0,
    initialMeasurementsCache: restored?.measurements ?? [],
  });
  const rows = virtualizer.getVirtualItems();

  useEffect(() => {
    itemCount.current = items.length;
  }, [items.length]);

  useEffect(
    () => () => {
      lastPosition = {
        key: positionKey,
        itemCount: itemCount.current,
        offset: virtualizer.scrollOffset ?? 0,
        measurements: virtualizer.takeSnapshot(),
      };
    },
    [positionKey, virtualizer],
  );
  const lastRendered = rows.at(-1)?.index ?? -1;

  useEffect(() => {
    const nearEnd = lastRendered >= items.length - LOAD_AHEAD_ROWS;
    // A failed page waits for Retry instead of being re-requested on every scroll.
    if (nearEnd && hasMore && !loadingMore && !loadMoreFailed) onLoadMore();
  }, [
    lastRendered,
    items.length,
    hasMore,
    loadingMore,
    loadMoreFailed,
    onLoadMore,
  ]);

  function onFocusCapture(event: FocusEvent<HTMLUListElement>) {
    if (!(event.target instanceof HTMLElement)) return;
    const index = Number(
      event.target.closest<HTMLLIElement>("li[data-index]")?.dataset.index,
    );
    setFocusedKey(keys[index] ?? null);
  }

  function onBlurCapture(event: FocusEvent<HTMLUListElement>) {
    const next = event.relatedTarget;
    if (!(next instanceof Node) || !event.currentTarget.contains(next)) {
      setFocusedKey(null);
    }
  }

  return (
    <ScrollArea
      viewportRef={viewportRef}
      className={modpackListStyles.scrollArea}
    >
      <ul
        className={modpackListStyles.list}
        // Rows are absolutely placed; only the visible window is rendered.
        style={{ height: virtualizer.getTotalSize() }}
        onFocusCapture={onFocusCapture}
        onBlurCapture={onBlurCapture}
      >
        {rows.map((row) => {
          const modpack = items[row.index];
          return (
            <li
              key={row.key}
              ref={virtualizer.measureElement}
              data-index={row.index}
              aria-posinset={row.index + 1}
              aria-setsize={hasMore ? -1 : items.length}
              className={modpackListStyles.row}
              style={{ transform: `translateY(${row.start}px)` }}
            >
              {modpack ? (
                <ModpackRow modpack={modpack} />
              ) : (
                <LoadMoreFooter failed={loadMoreFailed} onRetry={onLoadMore} />
              )}
            </li>
          );
        })}
      </ul>
    </ScrollArea>
  );
}

function LoadMoreFooter({
  failed,
  onRetry,
}: {
  failed: boolean;
  onRetry: () => void;
}) {
  if (failed) {
    return (
      <div role="alert" className={modpackListStyles.footer}>
        <span className={modpackListStyles.error}>
          Could not load more modpacks.
        </span>
        <Button type="button" variant="outline" size="sm" onClick={onRetry}>
          Retry
        </Button>
      </div>
    );
  }
  return (
    <div role="status" className={modpackListStyles.footer}>
      <Spinner aria-hidden />
      Loading more modpacks…
    </div>
  );
}
