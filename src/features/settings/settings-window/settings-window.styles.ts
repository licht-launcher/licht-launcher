export const settingsWindowStyles = {
  root: "flex h-svh flex-col overflow-hidden bg-home-sidebar text-popover-foreground",
  title:
    "flex shrink-0 items-center px-3.5 font-brand text-xs font-medium tracking-widest text-muted-foreground uppercase select-none",
  frame:
    "mx-2 mb-2 flex min-h-0 min-w-0 flex-1 overflow-hidden rounded-xl border border-foreground/10 bg-popover",
} as const;
