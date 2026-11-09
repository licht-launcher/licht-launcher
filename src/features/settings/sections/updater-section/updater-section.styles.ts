export const updaterSectionStyles = {
  row: "flex items-center justify-between gap-3",
  rowText: "flex min-w-0 flex-col gap-0.5",
  rowLabel: "text-sm font-normal",
  rowDescription: "text-sm text-muted-foreground",
  control: "shrink-0",
  statusRow: "flex flex-wrap items-center justify-between gap-3",
  statusText: "flex min-w-0 flex-1 flex-col gap-0.5 text-sm",
  statusReady: "text-green-500",
  statusIdle: "text-muted-foreground",
  error: "text-sm text-destructive",
  note: "text-xs text-muted-foreground",
} as const;
