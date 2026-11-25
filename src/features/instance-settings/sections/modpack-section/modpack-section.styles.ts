export const modpackSectionStyles = {
  root: "flex flex-col gap-6",
  summary: "flex flex-col gap-1 rounded-xl bg-background/60 p-4",
  summaryName: "truncate text-sm font-medium",
  summaryDetail: "truncate text-sm text-muted-foreground",
  group: "flex flex-col gap-2",
  groupText: "flex flex-col gap-1",
  label: "text-sm font-medium",
  hint: "text-sm text-muted-foreground",
  action: "self-start",
} as const;
