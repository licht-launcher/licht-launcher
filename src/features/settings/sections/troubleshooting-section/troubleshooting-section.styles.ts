export const troubleshootingSectionStyles = {
  root: "flex flex-col gap-8",
  row: "flex items-center justify-between gap-3",
  rowText: "flex min-w-0 flex-col gap-0.5",
  rowLabel: "text-sm font-normal",
  rowDescription: "text-sm text-muted-foreground",
  path: "truncate font-mono text-xs text-muted-foreground",
  details:
    "flex min-w-0 flex-col gap-0.5 font-mono text-xs text-muted-foreground",
  error: "text-sm text-destructive",
  control: "shrink-0",
} as const;
