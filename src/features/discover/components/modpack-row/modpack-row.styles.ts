export const modpackRowStyles = {
  link: "flex min-w-0 items-center gap-3 rounded-lg border border-border bg-card px-3 py-2.5 outline-none transition-colors hover:bg-muted focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50",
  icon: "size-14 shrink-0 rounded-lg object-cover",
  iconFallback:
    "flex size-14 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground [&_svg]:size-6",
  text: "flex min-w-0 flex-1 flex-col gap-1",
  title: "flex min-w-0 items-baseline gap-2",
  name: "truncate font-medium",
  author: "shrink-0 truncate text-xs text-muted-foreground",
  description: "truncate text-sm text-muted-foreground",
  footer: "flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 pt-0.5",
  tags: "flex min-w-0 flex-wrap items-center gap-1",
  meta: "ml-auto flex shrink-0 items-center gap-3 text-xs text-muted-foreground",
  stat: "flex items-center gap-1.5 tabular-nums [&_svg]:size-3.5",
  srOnly: "sr-only",
} as const;
