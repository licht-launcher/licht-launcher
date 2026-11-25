export const contentLoadingStyles = {
  /** Fills the content panel, which draws the frame. */
  root: "flex min-h-0 flex-1 flex-col overflow-hidden",
  tableWrapper: "w-full min-w-full",
  table: "w-full table-fixed text-sm",
  progress: "h-px rounded-none",
  scrollArea: "min-h-0 flex-1",
  row: "border-b border-border/60 last:border-b-0 odd:bg-black/10",
  cell: "px-3 py-2",
  item: "flex items-center gap-3 transition-opacity",
  iconPlaceholder: "size-8 shrink-0 animate-pulse rounded-md bg-muted",
  textPlaceholder: "min-w-0 flex-1 space-y-2",
  titlePlaceholder: "h-4 w-2/5 animate-pulse rounded bg-muted",
  subtitlePlaceholder: "h-3 w-3/5 animate-pulse rounded bg-muted",
  actionPlaceholder: "ml-auto h-8 w-20 animate-pulse rounded-md bg-muted",
} as const;
