export const settingsWindowStyles = {
  // Matches the former settings dialog: one popover surface, no frame.
  root: "relative flex h-svh overflow-hidden bg-popover text-sm text-popover-foreground",
  // An invisible title bar over the panel column, where the dialog had its
  // close button. It starts after the sidebar (`w-48`) so its drag region
  // never covers the section buttons.
  titleBar: "absolute top-0 right-0 left-48 z-10",
} as const;
