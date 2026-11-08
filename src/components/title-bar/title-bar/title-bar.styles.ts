export const titleBarStyles = {
  root: "pointer-events-auto relative shrink-0 text-popover-foreground",
  bar: "flex h-9 items-stretch",
  barNativeCaption: "pr-[var(--native-caption-inset)]",
  dragRegion: "min-w-0 flex-1",
  actions: "flex shrink-0 items-center gap-1.5 px-2 text-muted-foreground",
  chip: "h-6 max-w-36 gap-1.5 rounded-md border border-border/60 bg-muted/40 px-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground",
  updateChip: "cursor-pointer",
  updateIcon: "size-3.5 shrink-0",
  windowControls:
    "pointer-events-auto relative z-[60] flex shrink-0 items-stretch text-muted-foreground",
  control:
    "h-full w-14 shrink-0 rounded-none [&_svg:not([class*='size-'])]:size-4",
  closeControl: "hover:bg-destructive/20 hover:text-destructive",
  minimizeIcon: "size-4",
  closeIcon: "size-5",
} as const;
