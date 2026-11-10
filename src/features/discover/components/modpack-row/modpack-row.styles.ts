export const modpackRowStyles = {
  link: "flex min-w-0 items-center gap-3 rounded-lg px-3 py-2.5 outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50",
  icon: "size-12 shrink-0 rounded-lg object-cover",
  iconFallback:
    "flex size-12 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground [&_svg]:size-5",
  text: "flex min-w-0 flex-1 flex-col gap-0.5",
  title: "flex min-w-0 items-baseline gap-2",
  name: "truncate font-medium",
  author: "shrink-0 truncate text-xs text-muted-foreground",
  description: "line-clamp-2 text-sm text-muted-foreground",
  meta: "flex shrink-0 flex-col items-end gap-1 text-xs text-muted-foreground",
  downloads: "flex items-center gap-1 tabular-nums [&_svg]:size-3.5",
  srOnly: "sr-only",
} as const;
