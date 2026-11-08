export const contextMenuStyles = {
  content:
    "z-100 max-h-(--radix-context-menu-content-available-height) min-w-48 overflow-x-hidden overflow-y-auto rounded-lg border border-border bg-popover p-1 text-popover-foreground outline-none",
  item: "relative flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none select-none data-disabled:pointer-events-none data-disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  itemVariants: {
    default:
      "focus:bg-accent focus:text-accent-foreground data-highlighted:bg-accent data-highlighted:text-accent-foreground",
    destructive:
      "text-destructive focus:bg-destructive/10 data-highlighted:bg-destructive/10",
  },
  separator: "pointer-events-none -mx-1 my-1 h-px bg-border",
} as const;
