export const modpackContentCardStyles = {
  root: "flex shrink-0 flex-col rounded-lg border border-border/60 bg-card",
  main: "flex items-center gap-3 p-3",
  icon: "size-12 shrink-0 rounded-lg object-cover",
  iconFallback:
    "flex size-12 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground [&_svg]:size-6",
  heading: "flex min-w-0 flex-1 flex-col gap-1",
  title: "truncate text-base font-semibold",
  counts:
    "flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground",
  count: "flex items-center gap-1 tabular-nums [&_svg]:size-3.5",
  details:
    "flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-border/60 px-3 py-2 text-xs text-muted-foreground",
  detail: "flex min-w-0 items-center gap-1 [&_svg]:size-3.5 [&_svg]:shrink-0",
  providerIcon: "size-4 shrink-0 rounded-sm object-cover",
  name: "truncate font-medium text-foreground",
} as const;
