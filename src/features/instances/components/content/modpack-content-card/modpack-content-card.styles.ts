export const modpackContentCardStyles = {
  /** Part of the content panel: only a divider separates it from the extra content. */
  root: "flex shrink-0 flex-col border-b border-border/60",
  header: "flex items-start gap-3 px-3 py-4",
  icon: "size-12 shrink-0 rounded-md bg-muted object-cover",
  iconFallback:
    "flex size-12 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:size-6",
  heading: "flex min-w-0 flex-1 flex-col gap-1.5",
  titleLine: "flex min-w-0 items-baseline gap-2",
  title: "min-w-0 truncate text-base font-semibold",
  version: "max-w-40 shrink-0 truncate text-sm text-muted-foreground",
  subtitle:
    "flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1 text-sm text-muted-foreground [&_svg]:size-3.5 [&_svg]:shrink-0",
  subtitlePart: "flex min-w-0 items-center gap-1.5",
  counts:
    "flex flex-wrap items-center gap-x-4 gap-y-1 text-sm font-medium text-muted-foreground",
  count: "flex items-center gap-2 tabular-nums text-foreground",
} as const;
