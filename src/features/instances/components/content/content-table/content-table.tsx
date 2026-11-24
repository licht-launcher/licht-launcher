import { PackageIcon } from "lucide-react";
import { Fragment } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  type ContentActions,
  ContentRow,
} from "@/features/instances/components/content/content-row";
import { ContentTableColumns } from "@/features/instances/components/content/content-table-head";
import type { useContentVirtualizer } from "@/features/instances/hooks/use-content-virtualizer";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import { cn } from "@/lib/utils";

import { contentTableStyles } from "./content-table.styles";

export function ContentTable({
  entries,
  totalEntries,
  emptyLabel = "No content installed.",
  virtualization,
  actions,
}: {
  entries: InstanceContentEntry[];
  totalEntries: number;
  /** Shown when there is nothing to list before filters apply. */
  emptyLabel?: string;
  virtualization: ReturnType<typeof useContentVirtualizer>;
  /** Omitted for read-only lists, such as content a modpack manages. */
  actions?: ContentActions;
}) {
  const { viewportRef, virtualizer, virtualRows, tableEvents } = virtualization;
  const columns = actions ? 2 : 1;
  return (
    <ScrollArea
      className={contentTableStyles.scrollArea}
      viewportRef={viewportRef}
    >
      <div className={contentTableStyles.tableWrapper}>
        <table
          className={contentTableStyles.table}
          aria-label="Installed content"
          aria-rowcount={entries.length || undefined}
          {...tableEvents}
        >
          <ContentTableColumns actions={actions !== undefined} />
          <tbody>
            {entries.length === 0 ? (
              <tr>
                <td colSpan={columns}>
                  <div className={contentTableStyles.empty}>
                    <span className={contentTableStyles.emptyIconWell}>
                      <PackageIcon
                        aria-hidden
                        className={contentTableStyles.emptyIcon}
                      />
                    </span>
                    <p className={cn(contentTableStyles.emptyText)}>
                      {totalEntries === 0
                        ? emptyLabel
                        : "No content matches your filters."}
                    </p>
                  </div>
                </td>
              </tr>
            ) : (
              virtualRows.map((virtualRow, position) => {
                const entry = entries[virtualRow.index];
                const gap =
                  virtualRow.start - (virtualRows[position - 1]?.end ?? 0);
                return (
                  <Fragment key={virtualRow.key}>
                    {gap > 0 ? (
                      <tr aria-hidden="true" tabIndex={-1}>
                        <td
                          colSpan={columns}
                          className={contentTableStyles.emptyCell}
                          style={{ height: gap }}
                        />
                      </tr>
                    ) : null}
                    <ContentRow
                      entry={entry}
                      index={virtualRow.index}
                      last={virtualRow.index === entries.length - 1}
                      rowRef={virtualizer.measureElement}
                      actions={actions}
                    />
                  </Fragment>
                );
              })
            )}
            {virtualRows.length > 0 &&
            virtualizer.getTotalSize() >
              virtualRows[virtualRows.length - 1].end ? (
              <tr aria-hidden="true" tabIndex={-1}>
                <td
                  colSpan={columns}
                  className={contentTableStyles.emptyCell}
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
  );
}
