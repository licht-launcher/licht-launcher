export const settingsWindowStyles = {
  root: "relative flex h-svh overflow-hidden bg-popover text-sm text-popover-foreground",
  // Starts after the sidebar (`w-48`) so it never covers the section buttons.
  titleBar: "absolute top-0 right-0 left-48 z-10",
} as const;
