export const activeFiltersStyles = {
  root: "flex shrink-0 flex-wrap items-center gap-2 px-1 pb-2",
  remove:
    "cursor-pointer rounded-full outline-none transition-opacity hover:opacity-80 focus-visible:ring-3 focus-visible:ring-ring/50",
  chip: "h-7 gap-1.5 px-3 text-sm [&>img]:size-3.5 [&>svg]:size-3.5",
  removeIcon: "opacity-70",
} as const;
