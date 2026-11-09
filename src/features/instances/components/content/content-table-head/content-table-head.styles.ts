export const contentTableHeadStyles = {
  typeColumn: "w-28",
  actionsColumn: "w-44",
  head: "text-left text-xs font-medium tracking-wide text-muted-foreground",
  headRow: "border-b border-border/60",
  nameHeader: "px-3 py-3",
  typeHeader: "w-28 px-3 py-3",
  actionsHeader: "w-44 px-3 py-3 text-right",
  actionsHeaderContent: "flex items-center justify-end gap-1 font-medium",
  actionsHeaderIcon: "size-3.5",
  sortButton:
    "inline-flex cursor-pointer items-center gap-1 rounded-sm font-medium outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring",
  sortButtonActive: "text-foreground",
  sortIcon: "size-3.5 shrink-0",
  sortIconIdle: "size-3 shrink-0 text-muted-foreground opacity-60",
  headerLabel: "inline-flex items-center gap-1 font-medium",
} as const;
