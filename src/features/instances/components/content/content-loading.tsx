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
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border/60 bg-black/20">
      <div className="min-w-[48rem]">
        <table className="w-full table-fixed text-sm">
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
        className="h-px rounded-none"
      />
      <ScrollArea className="min-h-0 flex-1">
        <div className="min-w-[48rem]">
          <table className="w-full table-fixed text-sm">
            <ContentTableColumns />
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
