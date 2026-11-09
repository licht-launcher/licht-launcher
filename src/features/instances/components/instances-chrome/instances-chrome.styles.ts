export const instancesChromeStyles = {
  root: "flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
  frame:
    "mx-2 mt-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border border-foreground/10 bg-popover",
  content: "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-muted/30",
  sidebar:
    "flex h-full min-h-0 w-full flex-1 flex-col overflow-hidden bg-home-sidebar text-popover-foreground",
  sidebarPanel: "min-h-0 w-full flex-1 overflow-hidden",
  resizeHandle:
    "w-0 bg-transparent hover:scale-x-100 hover:bg-transparent focus-visible:scale-x-100 focus-visible:bg-transparent active:scale-x-100 active:bg-transparent",
  footer: "flex shrink-0 items-center justify-center px-3 pt-2 pb-1.5",
  footerGrid: "grid min-h-8 w-full grid-cols-4 items-center",
  footerStart: "flex items-center justify-start",
  footerIcon: "size-3.5",
  footerCenter: "col-span-2 flex min-w-0 items-center justify-center",
  footerEnd: "flex min-w-0 items-center justify-end",
  footerLabel: "truncate",
  footerAction:
    "h-8 gap-2 rounded-md p-0 text-xs text-muted-foreground hover:bg-transparent hover:text-foreground dark:hover:bg-transparent aria-expanded:bg-transparent aria-expanded:text-foreground",
  footerActionShrink: "min-w-0",
} as const;
