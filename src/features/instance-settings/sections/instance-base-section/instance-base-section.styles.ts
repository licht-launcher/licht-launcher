export const instanceBaseSectionStyles = {
  root: "flex flex-col gap-6",
  summary: "grid gap-4 rounded-xl bg-background/60 p-4 sm:grid-cols-2",
  summaryItem: "flex min-w-0 flex-col gap-2",
  summaryLabel: "flex items-center gap-2 text-sm text-muted-foreground",
  summaryIcon: "size-4 shrink-0",
  summaryName: "font-medium text-foreground",
  summaryValue:
    "flex h-8 items-center rounded-lg bg-muted/40 px-2.5 text-sm font-medium",
  busy: "flex items-center gap-2",
  busySpinner: "size-3.5",
  valueText: "truncate",
  group: "flex flex-col gap-2",
  groupText: "flex flex-col gap-1",
  label: "text-sm font-medium",
  hint: "text-sm text-muted-foreground",
  action: "self-start",
} as const;
