import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { toast } from "sonner";
import { Progress } from "@/components/ui/progress";
import { ContentLoading } from "@/features/instances/components/content/content-loading";
import { ContentRemovalDialog } from "@/features/instances/components/content/content-removal-dialog";
import { ContentTable } from "@/features/instances/components/content/content-table";
import {
  ContentTableColumns,
  ContentTableHead,
} from "@/features/instances/components/content/content-table-head";
import { useContentVirtualizer } from "@/features/instances/hooks/use-content-virtualizer";
import { instanceContentQuery } from "@/features/instances/lib/content-query";
import {
  type ContentFilter,
  type ContentSortDir,
  type ContentSortKey,
  sortContentEntries,
  toggleContentSort,
} from "@/features/instances/lib/content-table";
import {
  removeInstanceContent,
  setInstanceContentEnabled,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

export type { ContentFilter } from "@/features/instances/lib/content-table";
export const CONTENT_STALE_TIME = Number.POSITIVE_INFINITY;

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
  const disabled = update.isPending || remove.isPending;
  const virtualization = useContentVirtualizer({
    entries: sortedEntries,
    folder,
    filter,
    search,
    sortKey,
    sortDir,
    enabled: folder !== null && content.data !== undefined,
    disabled,
    removalOpen: pendingRemoval !== null,
  });

  if (folder === null)
    return (
      <p className="p-3 text-sm text-muted-foreground">
        Select an instance to manage its content.
      </p>
    );
  if (content.isPending)
    return <ContentLoading sortKey={sortKey} sortDir={sortDir} />;
  if (content.isError && content.data === undefined)
    return (
      <p className="p-3 text-sm text-destructive">
        Could not list instance content.
      </p>
    );

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
        <ContentTableColumns />
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
      <ContentTable
        entries={sortedEntries}
        totalEntries={content.data.length}
        virtualization={virtualization}
        disabled={disabled}
        onToggle={(entry, enabled) =>
          update.mutate({ kind: entry.kind, name: entry.name, enabled })
        }
        onRemove={setPendingRemoval}
      />
      <ContentRemovalDialog
        entry={pendingRemoval}
        pending={remove.isPending}
        onOpenChange={(open) => {
          if (!open) setPendingRemoval(null);
        }}
        onConfirm={(entry) =>
          remove.mutate(entry, { onSuccess: () => setPendingRemoval(null) })
        }
        onCloseAutoFocus={virtualization.onCloseAutoFocus}
      />
    </div>
  );
}
