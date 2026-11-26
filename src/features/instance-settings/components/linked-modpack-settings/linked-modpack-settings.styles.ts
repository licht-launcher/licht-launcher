// Rows follow the launcher settings cards.
export const linkedModpackSettingsStyles = {
  pack: "flex items-center gap-3",
  packText: "flex min-w-0 flex-1 flex-col",
  packName: "truncate text-sm font-medium",
  packVersion: "truncate text-sm",
  packDetails: "truncate text-xs text-muted-foreground",
  row: "flex items-center justify-between gap-3",
  rowText: "flex min-w-0 flex-col gap-0.5",
  rowLabel: "truncate text-sm font-normal",
  rowDescription: "text-sm text-muted-foreground",
  control: "shrink-0",
} as const;
