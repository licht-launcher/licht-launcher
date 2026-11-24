export const modpackContentCardStyles = {
  /** Part of the content panel: only a divider separates it from the extra content. */
  root: "flex shrink-0 items-start gap-3 border-b border-border/60 px-3 py-4",
  icon: "size-12 shrink-0 rounded-md bg-muted object-cover",
  iconFallback:
    "flex size-12 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:size-6",
  heading: "flex min-w-0 flex-1 flex-col gap-1.5",
  title: "truncate text-base font-semibold",
  counts:
    "flex flex-wrap items-center gap-x-4 gap-y-1 text-sm font-medium text-foreground",
  count: "flex items-center gap-2 tabular-nums",
  provider:
    "flex min-w-0 flex-wrap items-center gap-1.5 text-sm text-muted-foreground [&_svg]:size-3.5 [&_svg]:shrink-0",
  providerLink: "h-auto p-0 text-sm font-medium",
  providerName: "font-medium text-foreground",
  version: "text-muted-foreground",
} as const;
