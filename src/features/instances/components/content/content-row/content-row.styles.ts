export const contentRowStyles = {
  row: "w-full border-b border-border/60",
  rowLast: "border-b-0",
  rowOdd: "bg-black/10",
  cell: "px-3 py-2",
  item: "flex min-w-0 items-center gap-3",
  image: "size-8 shrink-0 rounded-md object-cover",
  text: "min-w-0",
  name: "truncate font-medium",
  detail: "truncate text-xs text-muted-foreground",
  actions: "flex items-center justify-end gap-2",
  removeButton:
    "text-muted-foreground opacity-60 hover:text-destructive hover:opacity-100",
  icon: "size-4",
} as const;
