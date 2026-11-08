export const gameVersionFieldStyles = {
  root: "flex flex-col gap-2",
  message: "text-sm text-muted-foreground",
  retryButton: "self-start",
  row: "flex items-center gap-3",
  combobox: "flex-1",
  snapshotsToggle:
    "flex shrink-0 items-center gap-2 text-sm text-muted-foreground",
  snapshotsSwitch: "data-[state=checked]:bg-warning",
} as const;
