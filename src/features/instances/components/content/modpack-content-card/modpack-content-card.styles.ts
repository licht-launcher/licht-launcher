export const modpackContentCardStyles = {
  /** A box of its own above the content list. */
  root: "flex shrink-0 items-start gap-3 rounded-lg border border-border/60 bg-black/20 p-4",
  icon: "size-12",
  heading: "flex min-w-0 flex-1 flex-col gap-3",
  title: "truncate text-base font-semibold",
  facts: "flex flex-wrap gap-x-10 gap-y-3",
  fact: "flex min-w-0 flex-col gap-1.5",
  value:
    "flex items-center gap-1.5 text-sm font-medium [&_svg]:size-3.5 [&_svg]:shrink-0 [&_svg]:text-foreground",
  versionFact: "flex min-w-0 max-w-48 flex-col gap-1.5",
  version:
    "flex min-w-0 items-center gap-1.5 text-sm font-medium [&_svg]:size-3.5 [&_svg]:shrink-0 [&_svg]:text-foreground",
  unknown:
    "flex items-center gap-1.5 text-sm text-muted-foreground [&_svg]:size-3.5 [&_svg]:shrink-0 [&_svg]:text-foreground",
  valueText: "truncate",
  label: "text-xs font-medium uppercase tracking-wide text-muted-foreground",
} as const;
