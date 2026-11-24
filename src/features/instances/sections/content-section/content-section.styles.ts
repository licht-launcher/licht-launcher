export const contentSectionStyles = {
  message: "p-3 text-sm text-muted-foreground",
  error: "p-3 text-sm text-destructive",
  toolbar:
    "flex shrink-0 items-center gap-2 border-b border-border/60 p-3 empty:hidden",
  extraTitle:
    "flex shrink-0 items-center gap-2 pr-2 text-sm font-semibold whitespace-nowrap",
  extraCount:
    "rounded-full bg-muted px-2 py-0.5 text-xs font-medium tabular-nums text-muted-foreground",
  root: "flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border/60 bg-black/20",
  headTable: "w-full shrink-0 table-fixed text-sm",
  progress: "h-px rounded-none",
} as const;
