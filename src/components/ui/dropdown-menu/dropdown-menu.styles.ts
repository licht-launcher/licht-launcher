export const dropdownMenuStyles = {
  content:
    "z-100 max-h-(--radix-dropdown-menu-content-available-height) min-w-48 origin-(--radix-dropdown-menu-content-transform-origin) overflow-x-hidden overflow-y-auto rounded-lg border border-border bg-popover p-1 text-popover-foreground outline-none motion-safe:data-open:animate-menu-in motion-safe:data-closed:animate-menu-out",
  item: "relative flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-xs font-medium outline-none select-none focus:bg-accent focus:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50 data-highlighted:bg-accent data-highlighted:text-accent-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  separator: "pointer-events-none mx-1 my-1 h-px bg-border",
  radioItem:
    "relative flex cursor-pointer items-center gap-2 rounded-md py-1.5 pr-2 pl-7 text-xs font-medium outline-none select-none focus:bg-accent focus:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50 data-highlighted:bg-accent data-highlighted:text-accent-foreground",
  radioIndicator: "absolute left-2 flex size-3.5 items-center justify-center",
  radioIcon: "size-3.5",
} as const;
