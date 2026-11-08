import { PackageIcon } from "lucide-react";
import { Fragment } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ContentRow } from "@/features/instances/components/content/content-row";
import { ContentTableColumns } from "@/features/instances/components/content/content-table-head";
import type { useContentVirtualizer } from "@/features/instances/hooks/use-content-virtualizer";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import { cn } from "@/lib/utils";

import { contentTableStyles } from "./content-table.styles";

export function ContentTable({
  entries,
  totalEntries,
  virtualization,
  disabled,
  onToggle,
  onRemove,
}: {
  entries: InstanceContentEntry[];
  totalEntries: number;
  virtualization: ReturnType<typeof useContentVirtualizer>;
  disabled: boolean;
  onToggle: (entry: InstanceContentEntry, enabled: boolean) => void;
  onRemove: (entry: InstanceContentEntry) => void;
}) {
  const { viewportRef, virtualizer, virtualRows, tableEvents } = virtualization;
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
          <ContentTableColumns />
          <tbody>
            {entries.length === 0 ? (
              <tr>
                <td colSpan={3}>
                  <div className={contentTableStyles.empty}>
                    <span className={contentTableStyles.emptyIconWell}>
                      <PackageIcon
                        aria-hidden
                        className={contentTableStyles.emptyIcon}
                      />
                    </span>
                    <p className={cn(contentTableStyles.emptyText)}>
                      {totalEntries === 0
                        ? "No content installed."
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
                          colSpan={3}
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
                      disabled={disabled}
                      onToggle={onToggle}
                      onRemove={onRemove}
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
                  colSpan={3}
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
