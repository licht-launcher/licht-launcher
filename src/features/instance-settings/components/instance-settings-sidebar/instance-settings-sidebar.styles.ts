// Matches the launcher settings sidebar.
export const instanceSettingsSidebarStyles = {
  root: "flex w-48 shrink-0 flex-col gap-4 border-r border-border px-3 pb-4",
  // Replaces the top padding; cancels the gap so the sections stay in place.
  dragRegion: "-mx-3 -mb-4 h-4 shrink-0",
  title: "truncate px-2.5 pt-1.5 text-sm font-semibold",
  nav: "flex flex-col gap-0",
  navButton:
    "h-auto flex items-center justify-start gap-2 px-2.5 py-1.75 font-medium",
  navIcon: "size-4 shrink-0",
  navLabel: "min-w-0 truncate",
  footer: "mt-auto flex min-w-0 flex-col",
  globalButton:
    "h-auto w-full justify-between px-2.5 py-1.75 font-medium text-muted-foreground",
} as const;
