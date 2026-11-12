export const modpackHeaderStyles = {
  root: "flex min-w-0 items-start gap-4",
  icon: "size-20 shrink-0 rounded-xl object-cover",
  iconFallback:
    "flex size-20 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground [&_svg]:size-8",
  heading: "flex min-w-0 flex-1 flex-col gap-1.5",
  titleLine: "flex min-w-0 items-baseline gap-2",
  title: "truncate text-xl font-semibold",
  author: "shrink-0 truncate text-sm text-muted-foreground",
  summary: "line-clamp-2 text-sm",
  stats:
    "flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground",
  stat: "flex items-center gap-1 tabular-nums [&_svg]:size-3.5",
  tags: "flex flex-wrap gap-1.5",
  actions: "flex shrink-0 items-center gap-2",
} as const;
