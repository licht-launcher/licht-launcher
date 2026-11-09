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

import { contentTableHeadStyles } from "./content-table-head.styles";

export function ContentTableColumns() {
  return (
    <colgroup>
      <col />
      <col className={contentTableHeadStyles.typeColumn} />
      <col className={contentTableHeadStyles.actionsColumn} />
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
          contentTableHeadStyles.head,
          showBorder && contentTableHeadStyles.headRow,
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
          className={contentTableHeadStyles.nameHeader}
        />
        <ContentSortableHeader
          label="Size"
          icon={BinaryIcon}
          column="size"
          sortKey={sortKey}
          sortDir={sortDir}
          onSort={onSort}
          interactive={interactive}
          className={contentTableHeadStyles.typeHeader}
        />
        <th className={contentTableHeadStyles.actionsHeader}>
          <span className={contentTableHeadStyles.actionsHeaderContent}>
            <Settings2Icon
              aria-hidden
              className={contentTableHeadStyles.actionsHeaderIcon}
              strokeWidth={2}
            />
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
            contentTableHeadStyles.sortButton,
            active && contentTableHeadStyles.sortButtonActive,
          )}
          onClick={() => {
            onSort(column);
          }}
        >
          <Icon
            aria-hidden
            className={contentTableHeadStyles.sortIcon}
            strokeWidth={2}
          />
          {label}
          {active ? (
            <SortIcon
              aria-hidden
              className={contentTableHeadStyles.sortIconIdle}
              strokeWidth={2}
            />
          ) : null}
        </button>
      ) : (
        <span className={contentTableHeadStyles.headerLabel}>
          <Icon
            aria-hidden
            className={contentTableHeadStyles.sortIcon}
            strokeWidth={2}
          />
          {label}
        </span>
      )}
    </th>
  );
}
