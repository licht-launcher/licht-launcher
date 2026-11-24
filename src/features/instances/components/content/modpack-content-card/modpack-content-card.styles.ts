export const modpackContentCardStyles = {
  root: "flex shrink-0 flex-col rounded-lg border border-border/60 bg-card",
  header: "flex items-center gap-3 px-5 py-4",
  icon: "size-10 shrink-0 rounded-lg object-cover",
  iconFallback:
    "flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground [&_svg]:size-5",
  heading: "flex min-w-0 flex-1 flex-col gap-1",
  titleLine: "flex min-w-0 items-baseline gap-2",
  title: "min-w-0 truncate text-lg font-semibold",
  version: "max-w-40 shrink-0 truncate text-sm text-muted-foreground",
  subtitle:
    "flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1 text-sm text-muted-foreground [&_svg]:size-3.5 [&_svg]:shrink-0",
  subtitlePart: "flex min-w-0 items-center gap-1.5",
  facts: "flex border-t border-border/60 px-5 py-4",
  includedFact: "flex min-w-0 flex-1 flex-col gap-1.5 overflow-hidden",
  factLabel:
    "text-xs font-medium uppercase tracking-wide text-muted-foreground",
  factValue: "flex items-center gap-x-3 whitespace-nowrap text-sm font-medium",
  count: "flex items-center gap-1.5 tabular-nums [&_svg]:size-4",
  countIcon: "text-muted-foreground",
  muted: "text-sm text-muted-foreground",
} as const;
