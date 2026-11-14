export const activeFiltersStyles = {
  root: "flex shrink-0 flex-wrap items-center gap-2 px-1 pb-2",
  // A block item would baseline-align the badge, and a text-first badge
  // (version) sits lower than an icon-first one (category).
  item: "flex",
  remove:
    "inline-flex cursor-pointer rounded-full outline-none transition-opacity hover:opacity-80 focus-visible:ring-3 focus-visible:ring-ring/50",
  removeIcon: "opacity-70",
  version: "border-border py-0 [&>svg]:translate-y-px",
} as const;
