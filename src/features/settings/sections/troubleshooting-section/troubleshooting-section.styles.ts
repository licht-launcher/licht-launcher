export const troubleshootingSectionStyles = {
  root: "flex flex-col gap-8",
  row: "flex items-center justify-between gap-3",
  rowStart: "flex items-start justify-between gap-3",
  rowText: "flex min-w-0 flex-col gap-0.5",
  rowLabel: "text-sm font-normal",
  rowDescription: "text-sm text-muted-foreground",
  path: "mt-1.5 truncate rounded-md bg-muted px-2 py-1 font-mono text-xs text-muted-foreground",
  details: "grid min-w-0 flex-1 grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 text-sm",
  term: "text-muted-foreground",
  definition: "min-w-0 truncate",
  error: "text-sm text-destructive",
  control: "shrink-0",
} as const;
