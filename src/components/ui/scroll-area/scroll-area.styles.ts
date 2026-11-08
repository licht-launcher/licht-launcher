export const scrollAreaStyles = {
  root: "relative",
  viewport:
    "size-full rounded-[inherit] transition-[color,box-shadow] outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-1",
  scrollbar:
    "flex touch-none p-px transition-colors select-none data-horizontal:h-2 data-horizontal:flex-col data-horizontal:border-t data-horizontal:border-t-transparent data-vertical:h-full data-vertical:w-2 data-vertical:border-l data-vertical:border-l-transparent",
  thumb:
    "relative flex-1 rounded-full bg-muted-foreground/18 transition-colors duration-150 hover:bg-muted-foreground/28",
} as const;
