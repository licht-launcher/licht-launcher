export const modpackContentCardStyles = {
  /** Part of the content panel: only a divider separates it from the extra content. */
  root: "flex shrink-0 items-start gap-3 border-b border-border/60 px-3 py-4",
  icon: "size-12 shrink-0 rounded-md bg-muted object-cover",
  iconFallback:
    "flex size-12 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:size-6",
  heading: "flex min-w-0 flex-1 flex-col gap-3",
  titleLine: "flex min-w-0 items-baseline gap-2",
  title: "min-w-0 truncate text-base font-semibold",
  version: "max-w-40 shrink-0 truncate text-sm text-muted-foreground",
  included: "flex flex-col gap-1.5",
  label: "text-xs font-medium uppercase tracking-wide text-muted-foreground",
  counts: "flex flex-wrap items-center gap-x-4 gap-y-1 text-sm font-medium",
  count: "flex items-center gap-2 tabular-nums",
} as const;
