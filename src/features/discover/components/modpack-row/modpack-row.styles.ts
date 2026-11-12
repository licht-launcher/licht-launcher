export const modpackRowStyles = {
  link: "flex min-w-0 items-start gap-3 rounded-lg px-3 py-2.5 outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50",
  icon: "size-14 shrink-0 rounded-lg object-cover",
  iconFallback:
    "flex size-14 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground [&_svg]:size-6",
  text: "flex min-w-0 flex-1 flex-col gap-1",
  title: "flex min-w-0 items-baseline gap-2",
  name: "truncate font-medium",
  author: "shrink-0 truncate text-xs text-muted-foreground",
  description: "line-clamp-2 text-sm text-muted-foreground",
  tags: "flex min-w-0 flex-wrap items-center gap-1 pt-0.5",
  meta: "flex shrink-0 flex-col items-end gap-1.5 pt-0.5 text-xs text-muted-foreground",
  stat: "flex items-center justify-end gap-1.5 tabular-nums [&_svg]:size-3.5",
  srOnly: "sr-only",
} as const;
