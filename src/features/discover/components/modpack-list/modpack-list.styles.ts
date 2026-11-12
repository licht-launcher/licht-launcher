export const modpackListStyles = {
  scrollArea: "min-h-0 flex-1",
  list: "relative w-full",
  // Half the gap above and below each card; also leaves room for the focus ring.
  row: "absolute top-0 left-0 w-full px-1 py-1",
  footer:
    "flex items-center justify-center gap-2 py-3 text-sm text-muted-foreground",
  error: "text-destructive",
} as const;
