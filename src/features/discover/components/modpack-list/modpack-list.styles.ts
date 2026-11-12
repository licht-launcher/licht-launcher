export const modpackListStyles = {
  scrollArea: "min-h-0 flex-1",
  list: "relative w-full",
  // Half the gap above and below each card and room for the focus ring; the
  // wider right side keeps the overlay scrollbar off the cards.
  row: "absolute top-0 left-0 w-full py-1 pr-4 pl-1",
  footer:
    "flex items-center justify-center gap-2 py-3 text-sm text-muted-foreground",
  error: "text-destructive",
} as const;
