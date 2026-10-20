import {
  defaultRangeExtractor,
  type Range,
  useVirtualizer,
} from "@tanstack/react-virtual";
import {
  type FocusEvent,
  type KeyboardEvent,
  useCallback,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type {
  ContentFilter,
  ContentSortDir,
  ContentSortKey,
} from "@/features/instances/lib/content-table";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

type ContentFocus = { key: string; action: "toggle" | "remove" };

export function useContentVirtualizer({
  entries,
  folder,
  filter,
  search,
  sortKey,
  sortDir,
  enabled,
  disabled,
  removalOpen,
}: {
  entries: InstanceContentEntry[];
  folder: string | null;
  filter: ContentFilter;
  search: string;
  sortKey: ContentSortKey;
  sortDir: ContentSortDir;
  enabled: boolean;
  disabled: boolean;
  removalOpen: boolean;
}) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const [focusedControl, setFocusedControl] = useState<ContentFocus | null>(
    null,
  );
  const [pendingFocus, setPendingFocus] = useState<ContentFocus | null>(null);
  const rowKeys = useMemo(
    () => entries.map((entry) => `${entry.kind}:${entry.name}`),
    [entries],
  );
  const rowIndexes = useMemo(
    () => new Map(rowKeys.map((key, index) => [key, index])),
    [rowKeys],
  );
  const focusedIndex = focusedControl
    ? (rowIndexes.get(focusedControl.key) ?? -1)
    : -1;
  const pendingIndex = pendingFocus
    ? (rowIndexes.get(pendingFocus.key) ?? -1)
    : -1;
  const getItemKey = useCallback((index: number) => rowKeys[index], [rowKeys]);
  const rangeExtractor = useCallback(
    (range: Range) => {
      const indexes = new Set(defaultRangeExtractor(range));
      if (focusedIndex >= 0) indexes.add(focusedIndex);
      if (pendingIndex >= 0) indexes.add(pendingIndex);
      return [...indexes].sort((left, right) => left - right);
    },
    [focusedIndex, pendingIndex],
  );
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLTableRowElement>({
    count: entries.length,
    getScrollElement: () => viewportRef.current,
    // This is only an initial geometry estimate; the rows retain their measured token-based sizes.
    estimateSize: () => 56,
    getItemKey,
    rangeExtractor,
    overscan: 5,
    enabled,
  });
  const virtualRows = virtualizer.getVirtualItems();

  useLayoutEffect(() => {
    const target = pendingFocus ?? focusedControl;
    const index = pendingFocus ? pendingIndex : focusedIndex;
    if (!target || index < 0) {
      if (pendingFocus) setPendingFocus(null);
      return;
    }
    if (!pendingFocus && document.activeElement !== document.body) return;
    const control = viewportRef.current?.querySelector<HTMLButtonElement>(
      `tr[data-index="${index}"] [data-content-action="${target.action}"]`,
    );
    if (control && !control.disabled) {
      control.focus({ preventScroll: true });
      if (pendingFocus) setPendingFocus(null);
    }
  }, [pendingFocus, focusedControl, pendingIndex, focusedIndex]);

  const view = useMemo(
    () => ({ folder, filter, search, sortKey, sortDir }),
    [folder, filter, search, sortKey, sortDir],
  );
  const previousView = useRef(view);
  useLayoutEffect(() => {
    if (previousView.current !== view) {
      previousView.current = view;
      virtualizer.scrollToOffset(0);
    }
  }, [virtualizer, view]);

  function onFocusCapture(event: FocusEvent<HTMLTableElement>) {
    if (!(event.target instanceof HTMLElement)) return;
    const row = event.target.closest<HTMLTableRowElement>("tr[data-index]");
    const action = event.target.closest<HTMLElement>("[data-content-action]")
      ?.dataset.contentAction;
    const key = rowKeys[Number(row?.dataset.index)];
    if (key && (action === "toggle" || action === "remove"))
      setFocusedControl({ key, action });
  }

  function onBlurCapture(event: FocusEvent<HTMLTableElement>) {
    if (removalOpen) return;
    if (
      !(event.relatedTarget instanceof Node) ||
      !event.currentTarget.contains(event.relatedTarget)
    )
      setFocusedControl(null);
  }

  function onKeyDownCapture(event: KeyboardEvent<HTMLTableElement>) {
    if (
      event.key !== "Tab" ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      disabled ||
      !(event.target instanceof HTMLElement)
    )
      return;
    const row = event.target.closest<HTMLTableRowElement>("tr[data-index]");
    const action = event.target.closest<HTMLElement>("[data-content-action]")
      ?.dataset.contentAction;
    if (!row || (event.shiftKey ? action !== "toggle" : action !== "remove"))
      return;
    const index = Number(row.dataset.index) + (event.shiftKey ? -1 : 1);
    if (index < 0 || index >= entries.length) return;
    event.preventDefault();
    setPendingFocus({
      key: rowKeys[index],
      action: event.shiftKey ? "remove" : "toggle",
    });
    virtualizer.scrollToIndex(index, { align: "auto" });
  }

  function onCloseAutoFocus(event: Event) {
    if (!focusedControl) return;
    const index = rowIndexes.get(focusedControl.key);
    if (index === undefined) return;
    event.preventDefault();
    setPendingFocus(focusedControl);
    virtualizer.scrollToIndex(index, { align: "auto" });
  }

  return {
    viewportRef,
    virtualizer,
    virtualRows,
    onCloseAutoFocus,
    tableEvents: { onFocusCapture, onBlurCapture, onKeyDownCapture },
  };
}
