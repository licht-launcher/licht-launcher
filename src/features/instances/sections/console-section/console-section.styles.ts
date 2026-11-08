export const consoleSectionStyles = {
  filterTrigger: "w-40 justify-between font-semibold",
  filterLabel: "flex min-w-0 items-center gap-1.5",
  filterLabelText: "truncate",
  filterChevron: "size-4 shrink-0 text-muted-foreground",
  filterContent: "w-40 p-1",
  filterTitle: "px-2 py-1.5 text-xs font-medium text-muted-foreground",
  filterOption:
    "flex w-full cursor-pointer items-center gap-1.5 rounded-md px-2 py-1.5 text-sm font-semibold outline-none hover:bg-accent",
  filterOptionIcon: "size-4 shrink-0",
  filterOptionLabel: "flex-1 text-left",
  root: "relative flex min-h-0 flex-1 flex-col",
  scrollArea: "min-h-0 flex-1 rounded-lg border border-border/60 bg-black/20",
  emptyLog:
    "flex flex-col gap-3 p-3 font-mono text-sm leading-relaxed text-muted-foreground",
  emptyLogPrompt:
    "select-none font-sans text-lg leading-none text-muted-foreground/70",
  logMessage: "p-3 font-mono text-sm leading-relaxed text-muted-foreground",
  log: "p-3 font-mono text-sm leading-relaxed",
  jumpButton: "absolute right-3 bottom-3 z-10 shadow-sm",
  selectable:
    "cursor-text select-text selection:bg-muted-foreground/35 selection:text-foreground",
  toneDefault: "text-foreground",
  toneInfo: "text-info",
  toneWarn: "text-warning",
  toneError: "text-destructive",
} as const;
