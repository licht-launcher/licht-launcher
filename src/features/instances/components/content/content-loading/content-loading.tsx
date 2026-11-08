import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  ContentTableColumns,
  ContentTableHead,
} from "@/features/instances/components/content/content-table-head";
import type {
  ContentSortDir,
  ContentSortKey,
} from "@/features/instances/lib/content-table";
import { cn } from "@/lib/utils";

import { contentLoadingStyles } from "./content-loading.styles";

const skeletonRows = ["one", "two", "three", "four", "five", "six"];
const skeletonOpacity = [
  "opacity-100",
  "opacity-85",
  "opacity-70",
  "opacity-55",
  "opacity-40",
  "opacity-25",
];

export function ContentLoading({
  sortKey,
  sortDir,
}: {
  sortKey: ContentSortKey;
  sortDir: ContentSortDir;
}) {
  return (
    <div className={contentLoadingStyles.root}>
      <div className={contentLoadingStyles.tableWrapper}>
        <table className={contentLoadingStyles.table}>
          <ContentTableColumns />
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
        className={contentLoadingStyles.progress}
      />
      <ScrollArea className={contentLoadingStyles.scrollArea}>
        <div className={contentLoadingStyles.tableWrapper}>
          <table className={contentLoadingStyles.table}>
            <ContentTableColumns />
            <tbody>
              {skeletonRows.map((row, index) => (
                <tr key={row} className={contentLoadingStyles.row}>
                  <td className={contentLoadingStyles.cell}>
                    <div
                      className={cn(
                        contentLoadingStyles.item,
                        skeletonOpacity[index],
                      )}
                    >
                      <div className={contentLoadingStyles.iconPlaceholder} />
                      <div className={contentLoadingStyles.textPlaceholder}>
                        <div
                          className={contentLoadingStyles.titlePlaceholder}
                        />
                        <div
                          className={contentLoadingStyles.subtitlePlaceholder}
                        />
                      </div>
                    </div>
                  </td>
                  <td className={contentLoadingStyles.cell}>
                    <div
                      className={cn(
                        contentLoadingStyles.metaPlaceholder,
                        skeletonOpacity[index],
                      )}
                    />
                  </td>
                  <td className={contentLoadingStyles.cell}>
                    <div
                      className={cn(
                        contentLoadingStyles.actionPlaceholder,
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
