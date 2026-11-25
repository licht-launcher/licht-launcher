import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { PackagePlusIcon } from "lucide-react";
import { type ReactNode, useMemo, useState } from "react";
import { toast } from "sonner";
import {
  removeInstanceContent,
  setInstanceContentEnabled,
} from "@/api/commands";
import { EmptyState } from "@/components/ui/empty-state";
import { Progress } from "@/components/ui/progress";
import { ContentLoading } from "@/features/instances/components/content/content-loading";
import { ContentRemovalDialog } from "@/features/instances/components/content/content-removal-dialog";
import { ContentTable } from "@/features/instances/components/content/content-table";
import {
  ContentTableColumns,
  ContentTableHead,
} from "@/features/instances/components/content/content-table-head";
import { ModpackContentCard } from "@/features/instances/components/content/modpack-content-card";
import { ModpackContentDialog } from "@/features/instances/components/content/modpack-content-dialog";
import { useContentVirtualizer } from "@/features/instances/hooks/use-content-virtualizer";
import { instanceContentQuery } from "@/features/instances/lib/content-query";
import {
  type ContentFilter,
  type ContentSortDir,
  type ContentSortKey,
  filterContentEntries,
  sortContentEntries,
  toggleContentSort,
} from "@/features/instances/lib/content-table";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";

import { contentSectionStyles } from "./content-section.styles";

export type { ContentFilter } from "@/features/instances/lib/content-table";
export const CONTENT_STALE_TIME = Number.POSITIVE_INFINITY;

export function ContentSection({
  folder,
  filter,
  search,
  modpack = null,
  linkedModpack = null,
  iconPath = null,
  toolbar = null,
}: {
  folder: string | null;
  filter: ContentFilter;
  search: string;
  modpack?: ModpackOrigin | null;
  /** The pack's store details saved at install, when the instance has them. */
  linkedModpack?: LinkedModpack | null;
  iconPath?: string | null;
  /** Search, filter and refresh controls, shown right above the list. */
  toolbar?: ReactNode;
}) {
  const queryClient = useQueryClient();
  const [pendingRemoval, setPendingRemoval] =
    useState<InstanceContentEntry | null>(null);
  const [sortKey, setSortKey] = useState<ContentSortKey>("name");
  const [sortDir, setSortDir] = useState<ContentSortDir>("asc");
  // Keyed by folder so switching instances closes the modpack content.
  const [modpackOpenFor, setModpackOpenFor] = useState<string | null>(null);
  const modpackOpen =
    modpack !== null && folder !== null && modpackOpenFor === folder;
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
  const modpackEntries = useMemo(
    () => (content.data ?? []).filter((entry) => entry.fromModpack),
    [content.data],
  );
  const listedEntries = useMemo(
    () => (content.data ?? []).filter((entry) => !entry.fromModpack),
    [content.data],
  );
  const sortedEntries = useMemo(
    () =>
      sortContentEntries(
        filterContentEntries(listedEntries, filter, search),
        sortDir,
      ),
    [listedEntries, filter, search, sortDir],
  );
  const disabled = update.isPending || remove.isPending;
  const toggleContent = (entry: InstanceContentEntry, enabled: boolean) =>
    update.mutate({ kind: entry.kind, name: entry.name, enabled });
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
      <p className={contentSectionStyles.message}>
        Select an instance to manage its content.
      </p>
    );
  function onSort(nextKey: ContentSortKey) {
    const next = toggleContentSort(sortKey, sortDir, nextKey);
    setSortKey(next.key);
    setSortDir(next.dir);
  }

  // Nothing to search, filter or refresh until the user adds content of their own.
  const noExtraContent =
    modpack !== null &&
    content.data !== undefined &&
    listedEntries.length === 0;

  return (
    <div className={contentSectionStyles.layout}>
      {modpack !== null && content.data !== undefined ? (
        <ModpackContentCard
          origin={modpack}
          linked={linkedModpack}
          iconPath={iconPath}
          entries={modpackEntries}
          onView={() => setModpackOpenFor(folder)}
        />
      ) : null}
      <div className={contentSectionStyles.root}>
        {noExtraContent ? null : (
          <div className={contentSectionStyles.toolbar}>
            {modpack !== null ? (
              <h3 className={contentSectionStyles.extraTitle}>
                Extra content{" "}
                {content.data !== undefined ? (
                  <span className={contentSectionStyles.extraCount}>
                    {listedEntries.length}
                  </span>
                ) : null}
              </h3>
            ) : null}
            {toolbar}
          </div>
        )}
        {content.isPending ? (
          <ContentLoading sortKey={sortKey} sortDir={sortDir} />
        ) : content.isError && content.data === undefined ? (
          <p className={contentSectionStyles.error}>
            Could not list instance content.
          </p>
        ) : (
          <>
            {content.isError ? (
              <p role="alert" className={contentSectionStyles.error}>
                Some content metadata could not be loaded. Refresh to try again.
              </p>
            ) : null}
            {noExtraContent ? (
              <EmptyState
                icon={<PackagePlusIcon />}
                title="No extra content installed"
                description="Mods and packs you add on top of the modpack appear here."
              />
            ) : (
              <>
                <table className={contentSectionStyles.headTable}>
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
                    className={contentSectionStyles.progress}
                  />
                ) : null}
                <ContentTable
                  entries={sortedEntries}
                  totalEntries={listedEntries.length}
                  virtualization={virtualization}
                  actions={{
                    disabled,
                    onToggle: toggleContent,
                    onRemove: setPendingRemoval,
                  }}
                />
              </>
            )}
            {modpack !== null ? (
              <ModpackContentDialog
                open={modpackOpen}
                onOpenChange={(open) => setModpackOpenFor(open ? folder : null)}
                origin={modpack}
                linked={linkedModpack}
                iconPath={iconPath}
                folder={folder}
                entries={modpackEntries}
                actions={{ disabled, onToggle: toggleContent }}
              />
            ) : null}
            <ContentRemovalDialog
              entry={pendingRemoval}
              pending={remove.isPending}
              onOpenChange={(open) => {
                if (!open) setPendingRemoval(null);
              }}
              onConfirm={(entry) =>
                remove.mutate(entry, {
                  onSuccess: () => setPendingRemoval(null),
                })
              }
              onCloseAutoFocus={virtualization.onCloseAutoFocus}
            />
          </>
        )}
      </div>
    </div>
  );
}
