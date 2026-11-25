import { PackagePlusIcon } from "lucide-react";
import { useMemo, useState } from "react";
import { EmptyState } from "@/components/ui/empty-state";
import { Progress } from "@/components/ui/progress";
import { ContentLoading } from "@/features/instances/components/content/content-loading";
import { ContentRemovalDialog } from "@/features/instances/components/content/content-removal-dialog";
import { ContentTable } from "@/features/instances/components/content/content-table";
import {
  ContentTableColumns,
  ContentTableHead,
} from "@/features/instances/components/content/content-table-head";
import { ContentToolbar } from "@/features/instances/components/content/content-toolbar";
import { ModpackContentCard } from "@/features/instances/components/content/modpack-content-card";
import { ModpackContentDialog } from "@/features/instances/components/content/modpack-content-dialog";
import { useContentVirtualizer } from "@/features/instances/hooks/use-content-virtualizer";
import { useInstanceContent } from "@/features/instances/hooks/use-instance-content";
import {
  type ContentFilter,
  type ContentSortDir,
  type ContentSortKey,
  filterContentEntries,
  sortContentEntries,
  toggleContentSort,
} from "@/features/instances/lib/content-table";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";

import { contentSectionStyles } from "./content-section.styles";

export type { ContentFilter } from "@/features/instances/lib/content-table";

export function ContentSection({
  folder,
  filter,
  onFilterChange,
  search,
  onSearchChange,
  modpack = null,
  linkedModpack = null,
  iconPath = null,
}: {
  folder: string | null;
  /** Search and filter live with the caller, so they survive switching instances. */
  filter: ContentFilter;
  onFilterChange: (filter: ContentFilter) => void;
  search: string;
  onSearchChange: (search: string) => void;
  modpack?: ModpackOrigin | null;
  /** The pack's details saved at install, when the instance has them. */
  linkedModpack?: LinkedModpack | null;
  iconPath?: string | null;
}) {
  const instanceContent = useInstanceContent(folder);
  const { content, modpackEntries, extraEntries, busy, toggle, remove } =
    instanceContent;
  const [pendingRemoval, setPendingRemoval] =
    useState<InstanceContentEntry | null>(null);
  const [sortKey, setSortKey] = useState<ContentSortKey>("name");
  const [sortDir, setSortDir] = useState<ContentSortDir>("asc");
  // Keyed by folder so switching instances closes the modpack content.
  const [modpackOpenFor, setModpackOpenFor] = useState<string | null>(null);
  const modpackOpen =
    modpack !== null && folder !== null && modpackOpenFor === folder;
  const sortedEntries = useMemo(
    () =>
      sortContentEntries(
        filterContentEntries(extraEntries, filter, search),
        sortDir,
      ),
    [extraEntries, filter, search, sortDir],
  );
  const virtualization = useContentVirtualizer({
    entries: sortedEntries,
    folder,
    filter,
    search,
    sortKey,
    sortDir,
    enabled: folder !== null && content.data !== undefined,
    disabled: busy,
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
    modpack !== null && content.data !== undefined && extraEntries.length === 0;

  return (
    <div className={contentSectionStyles.layout}>
      {/* Shown at once: until the list loads, the pack's files are unknown and View content waits. */}
      {modpack !== null ? (
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
                    {extraEntries.length}
                  </span>
                ) : null}
              </h3>
            ) : null}
            <ContentToolbar
              search={search}
              onSearchChange={onSearchChange}
              filter={filter}
              onFilterChange={onFilterChange}
              refreshing={instanceContent.refreshing}
              onRefresh={instanceContent.refresh}
            />
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
                  totalEntries={extraEntries.length}
                  virtualization={virtualization}
                  actions={{
                    disabled: busy,
                    onToggle: toggle,
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
                actions={{ disabled: busy, onToggle: toggle }}
              />
            ) : null}
            <ContentRemovalDialog
              entry={pendingRemoval}
              pending={remove.isPending}
              onOpenChange={(open) => {
                if (!open) setPendingRemoval(null);
              }}
              onConfirm={(entry) =>
                remove.mutate(
                  { folder, entry },
                  {
                    onSuccess: () => setPendingRemoval(null),
                  },
                )
              }
              onCloseAutoFocus={virtualization.onCloseAutoFocus}
            />
          </>
        )}
      </div>
    </div>
  );
}
