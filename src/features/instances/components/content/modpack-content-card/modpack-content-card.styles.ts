export const modpackContentCardStyles = {
  root: "flex shrink-0 flex-col rounded-lg border border-border/60 bg-card",
  header: "flex items-center gap-4 px-5 py-4",
  icon: "size-14 shrink-0 rounded-xl object-cover",
  iconFallback:
    "flex size-14 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground [&_svg]:size-7",
  heading: "flex min-w-0 flex-1 flex-col gap-1",
  title: "truncate text-lg font-semibold",
  subtitle:
    "flex min-w-0 items-center gap-1.5 truncate text-sm text-muted-foreground [&_svg]:size-3.5 [&_svg]:shrink-0",
  facts:
    "grid grid-cols-3 gap-6 border-t border-border/60 px-5 py-4 max-sm:grid-cols-1 max-sm:gap-3",
  fact: "flex min-w-0 flex-col gap-1.5",
  factLabel:
    "text-xs font-medium uppercase tracking-wide text-muted-foreground",
  factValue:
    "flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-sm font-medium",
  count: "flex items-center gap-1.5 tabular-nums [&_svg]:size-4",
  countIcon: "text-muted-foreground",
  muted: "text-sm text-muted-foreground",
} as const;
