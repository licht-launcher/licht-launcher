import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  defaultRangeExtractor,
  type Range,
  useVirtualizer,
} from "@tanstack/react-virtual";
import {
  AlertTriangleIcon,
  ArrowDownAZIcon,
  ArrowDownIcon,
  ArrowUpIcon,
  BinaryIcon,
  BlocksIcon,
  ImageIcon,
  type LucideIcon,
  PackageIcon,
  Settings2Icon,
  SparklesIcon,
  Trash2Icon,
} from "lucide-react";
import {
  Fragment,
  useCallback,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { instanceContentQuery } from "@/features/instances/lib/content-query";
import {
  removeInstanceContent,
  setInstanceContentEnabled,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import { cn } from "@/lib/utils";

const skeletonRows = ["one", "two", "three", "four", "five", "six"];
const skeletonOpacity = [
  "opacity-100",
  "opacity-85",
  "opacity-70",
  "opacity-55",
  "opacity-40",
  "opacity-25",
];
export const CONTENT_STALE_TIME = Number.POSITIVE_INFINITY;
type ContentSortKey = "name" | "size";
type ContentSortDir = "asc" | "desc";
type ContentFocus = { key: string; action: "toggle" | "remove" };

function formatSize(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${Math.round(size / 1024)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

function formatContentTitle(name: string): string {
  const withoutExtension = name.replace(/\.[^.]+$/, "");
  return withoutExtension
    .replace(/[-_.]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function ContentIcon({ kind }: { kind: InstanceContentKind }) {
  const contentIcon =
    kind === "mods"
      ? {
          Icon: BlocksIcon,
          wrapperClassName: "bg-accent-violet/15",
          iconClassName: "text-accent-violet",
        }
      : kind === "resourcePacks"
        ? {
            Icon: ImageIcon,
            wrapperClassName: "bg-accent-orange/15",
            iconClassName: "text-accent-orange",
          }
        : {
            Icon: SparklesIcon,
            wrapperClassName: "bg-accent-teal/15",
            iconClassName: "text-accent-teal",
          };
  const { Icon, wrapperClassName, iconClassName } = contentIcon;

  return (
    <span
      className={cn(
        "flex size-8 shrink-0 items-center justify-center rounded-md",
        wrapperClassName,
      )}
    >
      <Icon aria-hidden className={cn("size-4", iconClassName)} />
    </span>
  );
}

function sortContentEntries(
  entries: InstanceContentEntry[],
  key: ContentSortKey,
  dir: ContentSortDir,
): InstanceContentEntry[] {
  const sign = dir === "asc" ? 1 : -1;
  return [...entries].sort((left, right) => {
    const compare =
      key === "size"
        ? left.size - right.size
        : left.title.localeCompare(right.title, undefined, {
            sensitivity: "base",
          });
    return compare * sign;
  });
}

function toggleContentSort(
  currentKey: ContentSortKey,
  currentDir: ContentSortDir,
  nextKey: ContentSortKey,
): { key: ContentSortKey; dir: ContentSortDir } {
  if (currentKey === nextKey) {
    return { key: currentKey, dir: currentDir === "asc" ? "desc" : "asc" };
  }
  return { key: nextKey, dir: "asc" };
}

function ContentTableHead({
  sortKey,
  sortDir,
  onSort,
  interactive = true,
  showBorder = true,
}: {
  sortKey: ContentSortKey;
  sortDir: ContentSortDir;
  onSort: (key: ContentSortKey) => void;
  interactive?: boolean;
  showBorder?: boolean;
}) {
  return (
    <thead>
      <tr
        className={cn(
          "text-left text-xs font-medium tracking-wide text-muted-foreground",
          showBorder && "border-b border-border/60",
        )}
      >
        <ContentSortableHeader
          label="Content"
          icon={ArrowDownAZIcon}
          column="name"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          interactive={interactive}
          className="px-3 py-3"
        />
        <ContentSortableHeader
          label="Size"
          icon={BinaryIcon}
          column="size"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          interactive={interactive}
          className="w-28 px-3 py-3"
        />
        <th className="w-44 px-3 py-3 text-right">
          <span className="flex items-center justify-end gap-1 font-medium">
            <Settings2Icon aria-hidden className="size-3.5" strokeWidth={2} />
            Actions
          </span>
        </th>
      </tr>
    </thead>
  );
}

function ContentSortableHeader({
  label,
  icon: Icon,
  column,
  sortKey,
  sortDir,
  onSort,
  interactive,
  className,
}: {
  label: string;
  icon: LucideIcon;
  column: ContentSortKey;
  sortKey: ContentSortKey;
  sortDir: ContentSortDir;
  onSort: (key: ContentSortKey) => void;
  interactive: boolean;
  className?: string;
}) {
  const active = sortKey === column;
  const SortIcon = sortDir === "desc" ? ArrowDownIcon : ArrowUpIcon;

  return (
    <th className={className} aria-sort={active ? `${sortDir}ending` : "none"}>
      {interactive ? (
        <button
          type="button"
          className={cn(
            "inline-flex cursor-pointer items-center gap-1 rounded-sm font-medium outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
            active && "text-foreground",
          )}
          onClick={() => {
            onSort(column);
          }}
        >
          <Icon aria-hidden className="size-3.5 shrink-0" strokeWidth={2} />
          {label}
          {active ? (
            <SortIcon
              aria-hidden
              className="size-3 shrink-0 text-muted-foreground opacity-60"
              strokeWidth={2}
            />
          ) : null}
        </button>
      ) : (
        <span className="inline-flex items-center gap-1 font-medium">
          <Icon aria-hidden className="size-3.5 shrink-0" strokeWidth={2} />
          {label}
        </span>
      )}
    </th>
  );
}

export type ContentFilter = "all" | InstanceContentKind;

export function ContentSection({
  folder,
  filter,
  search,
}: {
  folder: string | null;
  filter: ContentFilter;
  search: string;
}) {
  const queryClient = useQueryClient();
  const [pendingRemoval, setPendingRemoval] =
    useState<InstanceContentEntry | null>(null);
  const [sortKey, setSortKey] = useState<ContentSortKey>("name");
  const [sortDir, setSortDir] = useState<ContentSortDir>("asc");
  const viewportRef = useRef<HTMLDivElement>(null);
  const [focusedControl, setFocusedControl] = useState<ContentFocus | null>(
    null,
  );
  const [pendingFocus, setPendingFocus] = useState<ContentFocus | null>(null);
  const content = useQuery({
    ...instanceContentQuery(queryClient, folder ?? ""),
    enabled: folder !== null,
    staleTime: CONTENT_STALE_TIME,
  });
  const update = useMutation({
    mutationFn: (input: {
      kind: InstanceContentKind;
      name: string;
      enabled: boolean;
    }) =>
      setInstanceContentEnabled(
        folder ?? "",
        input.kind,
        input.name,
        input.enabled,
      ),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["instance-content", folder],
      });
      toast.success("Content status updated.");
    },
    onError: (error) => toast.error(friendlyError(messageOf(error))),
  });
  const remove = useMutation({
    mutationFn: (entry: InstanceContentEntry) =>
      removeInstanceContent(
        folder ?? "",
        entry.kind,
        entry.name,
        entry.enabled,
      ),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["instance-content", folder],
      });
      toast.success("Content removed.");
    },
    onError: (error) => toast.error(friendlyError(messageOf(error))),
  });

  const sortedEntries = useMemo(() => {
    const normalizedSearch = search.trim().toLocaleLowerCase();
    const entries = (content.data ?? []).filter(
      (entry) =>
        (filter === "all" || entry.kind === filter) &&
        (normalizedSearch === "" ||
          [entry.title, entry.name, entry.version ?? ""].some((value) =>
            value.toLocaleLowerCase().includes(normalizedSearch),
          )),
    );
    return sortContentEntries(entries, sortKey, sortDir);
  }, [content.data, filter, search, sortKey, sortDir]);
  const rowKeys = useMemo(
    () => sortedEntries.map((entry) => `${entry.kind}:${entry.name}`),
    [sortedEntries],
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
    count: sortedEntries.length,
    getScrollElement: () => viewportRef.current,
    // This is only an initial geometry estimate; the rows retain their measured token-based sizes.
    estimateSize: () => 56,
    getItemKey,
    rangeExtractor,
    overscan: 5,
    enabled: folder !== null && content.data !== undefined,
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

  if (folder === null) {
    return (
      <p className="p-3 text-sm text-muted-foreground">
        Select an instance to manage its content.
      </p>
    );
  }
  if (content.isPending) {
    return (
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border/60 bg-black/20">
        <div className="min-w-[48rem]">
          <table className="w-full table-fixed text-sm">
            <colgroup>
              <col />
              <col className="w-28" />
              <col className="w-44" />
            </colgroup>
            <ContentTableHead
              sortKey={sortKey}
              sortDir={sortDir}
              onSort={() => {}}
              interactive={false}
              showBorder={false}
            />
          </table>
        </div>
        <Progress
          aria-label="Loading content metadata"
          className="h-px rounded-none"
        />
        <ScrollArea className="min-h-0 flex-1">
          <div className="min-w-[48rem]">
            <table className="w-full table-fixed text-sm">
              <colgroup>
                <col />
                <col className="w-28" />
                <col className="w-44" />
              </colgroup>
              <tbody>
                {skeletonRows.map((row, index) => (
                  <tr
                    key={row}
                    className="border-b border-border/60 last:border-b-0 odd:bg-black/10"
                  >
                    <td className="px-3 py-2">
                      <div
                        className={cn(
                          "flex items-center gap-3 transition-opacity",
                          skeletonOpacity[index],
                        )}
                      >
                        <div className="size-8 shrink-0 animate-pulse rounded-md bg-muted" />
                        <div className="min-w-0 flex-1 space-y-2">
                          <div className="h-4 w-2/5 animate-pulse rounded bg-muted" />
                          <div className="h-3 w-3/5 animate-pulse rounded bg-muted" />
                        </div>
                      </div>
                    </td>
                    <td className="px-3 py-2">
                      <div
                        className={cn(
                          "h-3 w-16 animate-pulse rounded bg-muted",
                          skeletonOpacity[index],
                        )}
                      />
                    </td>
                    <td className="px-3 py-2">
                      <div
                        className={cn(
                          "ml-auto h-8 w-20 animate-pulse rounded-md bg-muted",
                          skeletonOpacity[index],
                        )}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </ScrollArea>
      </div>
    );
  }
  if (content.isError && content.data === undefined) {
    return (
      <p className="p-3 text-sm text-destructive">
        Could not list instance content.
      </p>
    );
  }

  function onSort(nextKey: ContentSortKey) {
    const next = toggleContentSort(sortKey, sortDir, nextKey);
    setSortKey(next.key);
    setSortDir(next.dir);
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border/60 bg-black/20">
      {content.isError ? (
        <p role="alert" className="p-3 text-sm text-destructive">
          Some content metadata could not be loaded. Refresh to try again.
        </p>
      ) : null}
      <table className="w-full shrink-0 table-fixed text-sm">
        <colgroup>
          <col />
          <col className="w-28" />
          <col className="w-44" />
        </colgroup>
        <ContentTableHead
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          showBorder={!content.isFetching}
        />
      </table>
      {content.isFetching ? (
        <Progress
          aria-label="Loading content metadata"
          className="h-px rounded-none"
        />
      ) : null}
      <ScrollArea className="min-h-0 flex-1" viewportRef={viewportRef}>
        <div className="min-w-[48rem]">
          <table
            className="w-full table-fixed text-sm"
            aria-label="Installed content"
            aria-rowcount={sortedEntries.length || undefined}
            onFocusCapture={(event) => {
              if (!(event.target instanceof HTMLElement)) return;
              const row =
                event.target.closest<HTMLTableRowElement>("tr[data-index]");
              const action = event.target.closest<HTMLElement>(
                "[data-content-action]",
              )?.dataset.contentAction;
              const key = rowKeys[Number(row?.dataset.index)];
              if (key && (action === "toggle" || action === "remove"))
                setFocusedControl({ key, action });
            }}
            onBlurCapture={(event) => {
              if (pendingRemoval !== null) return;
              if (
                !(event.relatedTarget instanceof Node) ||
                !event.currentTarget.contains(event.relatedTarget)
              )
                setFocusedControl(null);
            }}
            onKeyDownCapture={(event) => {
              if (
                event.key !== "Tab" ||
                event.altKey ||
                event.ctrlKey ||
                event.metaKey ||
                update.isPending ||
                remove.isPending ||
                !(event.target instanceof HTMLElement)
              )
                return;
              const row =
                event.target.closest<HTMLTableRowElement>("tr[data-index]");
              const action = event.target.closest<HTMLElement>(
                "[data-content-action]",
              )?.dataset.contentAction;
              if (
                !row ||
                (event.shiftKey ? action !== "toggle" : action !== "remove")
              )
                return;
              const index =
                Number(row.dataset.index) + (event.shiftKey ? -1 : 1);
              if (index < 0 || index >= sortedEntries.length) return;
              event.preventDefault();
              setPendingFocus({
                key: rowKeys[index],
                action: event.shiftKey ? "remove" : "toggle",
              });
              virtualizer.scrollToIndex(index, { align: "auto" });
            }}
          >
            <colgroup>
              <col />
              <col className="w-28" />
              <col className="w-44" />
            </colgroup>
            <tbody>
              {sortedEntries.length === 0 ? (
                <tr>
                  <td colSpan={3}>
                    <div className="flex min-h-56 flex-col items-center justify-center gap-3 px-4 text-center">
                      <span className="flex size-12 items-center justify-center rounded-full bg-muted text-muted-foreground">
                        <PackageIcon aria-hidden className="size-6" />
                      </span>
                      <p
                        className={cn(
                          "font-brand text-sm font-medium leading-normal text-muted-foreground",
                        )}
                      >
                        {content.data.length === 0
                          ? "No content installed."
                          : "No content matches your filters."}
                      </p>
                    </div>
                  </td>
                </tr>
              ) : (
                virtualRows.map((virtualRow, position) => {
                  const entry = sortedEntries[virtualRow.index];
                  const gap =
                    virtualRow.start - (virtualRows[position - 1]?.end ?? 0);
                  return (
                    <Fragment key={virtualRow.key}>
                      {gap > 0 ? (
                        <tr aria-hidden="true" tabIndex={-1}>
                          <td
                            colSpan={3}
                            className="border-0 p-0"
                            style={{ height: gap }}
                          />
                        </tr>
                      ) : null}
                      <tr
                        ref={virtualizer.measureElement}
                        data-index={virtualRow.index}
                        aria-rowindex={virtualRow.index + 1}
                        className={cn(
                          "border-b border-border/60",
                          virtualRow.index === sortedEntries.length - 1 &&
                            "border-b-0",
                          virtualRow.index % 2 === 0 && "bg-black/10",
                        )}
                      >
                        <td className="px-3 py-2">
                          <div className="flex min-w-0 items-center gap-3">
                            {entry.iconUrl ? (
                              <img
                                src={entry.iconUrl}
                                alt=""
                                className="size-8 shrink-0 rounded-md object-cover"
                              />
                            ) : (
                              <ContentIcon kind={entry.kind} />
                            )}
                            <div className="min-w-0">
                              <p
                                className="truncate font-medium"
                                title={entry.title}
                              >
                                {entry.title}
                              </p>
                              <p
                                className="truncate text-xs text-muted-foreground"
                                title={entry.name}
                              >
                                {entry.version ? `${entry.version} · ` : ""}
                                {entry.name}
                              </p>
                            </div>
                          </div>
                        </td>
                        <td className="px-3 py-2 text-xs text-muted-foreground">
                          {formatSize(entry.size)}
                        </td>
                        <td className="px-3 py-2">
                          <div className="flex items-center justify-end gap-2">
                            <Switch
                              data-content-action="toggle"
                              checked={entry.enabled}
                              disabled={update.isPending || remove.isPending}
                              aria-label={`${entry.enabled ? "Disable" : "Enable"} ${entry.title}`}
                              onCheckedChange={(enabled) =>
                                update.mutate({
                                  kind: entry.kind,
                                  name: entry.name,
                                  enabled,
                                })
                              }
                            />
                            <Button
                              data-content-action="remove"
                              type="button"
                              size="icon"
                              variant="ghost"
                              className="text-muted-foreground opacity-60 hover:text-destructive hover:opacity-100"
                              aria-label={`Remove ${formatContentTitle(entry.name)}`}
                              disabled={update.isPending || remove.isPending}
                              onClick={() => setPendingRemoval(entry)}
                            >
                              <Trash2Icon aria-hidden className="size-4" />
                            </Button>
                          </div>
                        </td>
                      </tr>
                    </Fragment>
                  );
                })
              )}
              {virtualRows.length > 0 &&
              virtualizer.getTotalSize() >
                virtualRows[virtualRows.length - 1].end ? (
                <tr aria-hidden="true" tabIndex={-1}>
                  <td
                    colSpan={3}
                    className="border-0 p-0"
                    style={{
                      height:
                        virtualizer.getTotalSize() -
                        virtualRows[virtualRows.length - 1].end,
                    }}
                  />
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
      </ScrollArea>
      <Dialog
        open={pendingRemoval !== null}
        onOpenChange={(open) => {
          if (!open) setPendingRemoval(null);
        }}
      >
        <DialogContent
          onCloseAutoFocus={(event) => {
            if (!focusedControl) return;
            const index = rowIndexes.get(focusedControl.key);
            if (index === undefined) return;
            event.preventDefault();
            setPendingFocus(focusedControl);
            virtualizer.scrollToIndex(index, { align: "auto" });
          }}
        >
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertTriangleIcon className="size-5 text-destructive" />
              Remove content?
            </DialogTitle>
            <DialogDescription>
              This will permanently delete{" "}
              <strong>{pendingRemoval?.name}</strong> from this instance. This
              action cannot be undone by the launcher.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">
                Cancel
              </Button>
            </DialogClose>
            <Button
              type="button"
              variant="destructive"
              disabled={remove.isPending || pendingRemoval === null}
              onClick={() => {
                if (pendingRemoval !== null) {
                  remove.mutate(pendingRemoval, {
                    onSuccess: () => setPendingRemoval(null),
                  });
                }
              }}
            >
              Remove content
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
