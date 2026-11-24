export const modpackContentCardStyles = {
  /** Part of the content panel: only a divider separates it from the extra content. */
  root: "flex shrink-0 items-start gap-3 border-b border-border/60 px-3 py-4",
  icon: "size-12 shrink-0 rounded-md bg-muted object-cover",
  iconFallback:
    "flex size-12 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:size-6",
  heading: "flex min-w-0 flex-1 flex-col gap-3",
  title: "truncate text-base font-semibold",
  facts: "flex flex-wrap gap-x-10 gap-y-3",
  fact: "flex min-w-0 flex-col gap-1.5",
  value: "text-sm font-medium",
  versionFact: "flex min-w-0 max-w-48 flex-col gap-1.5",
  version: "truncate text-sm font-medium",
  unknown: "text-sm text-muted-foreground",
  label: "text-xs font-medium uppercase tracking-wide text-muted-foreground",
} as const;
