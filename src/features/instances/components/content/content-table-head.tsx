import {
  ArrowDownAZIcon,
  ArrowDownIcon,
  ArrowUpIcon,
  BinaryIcon,
  type LucideIcon,
  Settings2Icon,
} from "lucide-react";
import type {
  ContentSortDir,
  ContentSortKey,
} from "@/features/instances/lib/content-table";
import { cn } from "@/lib/utils";

export function ContentTableColumns() {
  return (
    <colgroup>
      <col />
      <col className="w-28" />
      <col className="w-44" />
    </colgroup>
  );
}

export function ContentTableHead({
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
