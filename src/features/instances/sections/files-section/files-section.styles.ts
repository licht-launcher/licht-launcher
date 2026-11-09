export const filesSectionStyles = {
  breadcrumbWrapper: "min-w-0 flex-1",
  breadcrumbs:
    "flex min-w-0 flex-wrap items-center gap-1 text-xs font-medium text-muted-foreground",
  breadcrumbItem: "min-w-0",
  breadcrumbButton:
    "cursor-pointer rounded-sm outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
  breadcrumbCurrent: "text-foreground",
  breadcrumbSegment: "flex min-w-0 items-center gap-1",
  breadcrumbIcon: "size-3.5 shrink-0",
  breadcrumbLink:
    "cursor-pointer truncate rounded-sm outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
  root: "flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border/60 bg-black/20",
  message: "p-3 text-sm leading-normal text-muted-foreground",
  tableWrapper: "min-w-[48rem]",
  table: "w-full table-fixed text-sm leading-normal",
  sizeColumn: "w-24",
  dateColumn: "w-44",
  progress: "h-px rounded-none",
  scrollArea: "min-h-0 flex-1",
  skeletonRow: "border-b border-border/60 last:border-b-0 odd:bg-black/10",
  skeletonCell: "px-3 py-3",
  skeletonItem: "flex items-center gap-2 transition-opacity",
  skeletonIcon: "size-4 animate-pulse rounded bg-muted",
  skeletonName: "h-4 w-2/5 animate-pulse rounded bg-muted",
  skeletonSize: "h-3 w-12 animate-pulse rounded bg-muted",
  skeletonDate: "h-3 w-28 animate-pulse rounded bg-muted",
  errorBlock: "flex flex-col gap-2 p-3",
  errorText: "text-sm leading-normal text-destructive",
  retryButton: "self-start",
  headTable:
    "w-full shrink-0 table-fixed caption-bottom text-sm leading-normal",
  bodyWrapper: "flex flex-col",
  bodyTable: "w-full table-fixed caption-bottom text-sm leading-normal",
  headRow:
    "border-b border-border/60 text-left text-xs font-medium tracking-wide text-muted-foreground",
  sizeHeader: "w-24 px-3 py-3",
  dateHeader: "w-44 px-3 py-3",
  sortButton:
    "inline-flex cursor-pointer items-center gap-1 rounded-sm font-medium outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
  sortIconIdle: "size-3 shrink-0 text-muted-foreground opacity-60",
  empty:
    "flex flex-col items-center justify-center gap-3 px-3 py-16 text-muted-foreground",
  emptyIconWell:
    "flex size-14 shrink-0 items-center justify-center rounded-full bg-muted",
  emptyIcon: "size-6 shrink-0",
  emptyText: "font-brand text-sm font-medium leading-normal",
  parentRow: "border-b border-border/60 hover:bg-muted/40",
  rowStriped: "bg-black/15",
  entryButton:
    "flex w-full min-w-0 cursor-pointer items-center gap-2 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring",
  entryIcon: "size-4 shrink-0 text-muted-foreground",
  entryName: "min-w-0 truncate font-medium text-foreground",
  metaCell: "px-3 py-3 text-xs text-muted-foreground tabular-nums",
  row: "border-b border-border/60 last:border-b-0",
  rowDirectory: "cursor-pointer hover:bg-muted/40",
  rowFile: "hover:bg-muted/30",
  entryContent: "flex min-w-0 items-center gap-2",
} as const;
