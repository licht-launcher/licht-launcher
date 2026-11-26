// Rows follow the launcher settings cards.
export const instanceBaseSectionStyles = {
  root: "flex flex-col gap-8",
  row: "flex items-center justify-between gap-3",
  rowText: "flex min-w-0 flex-col gap-0.5",
  rowName: "flex min-w-0 items-center gap-2",
  rowLabel: "text-sm font-normal",
  rowDescription: "text-sm text-muted-foreground",
  icon: "size-4 shrink-0 text-muted-foreground",
  value: "min-w-0 truncate text-sm font-medium",
  busy: "flex shrink-0 items-center gap-2 text-sm font-medium",
  busySpinner: "size-3.5",
  control: "shrink-0",
} as const;
