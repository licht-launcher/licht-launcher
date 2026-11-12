export const modpackSidebarStyles = {
  root: "flex min-w-0 flex-col gap-4 pr-3",
  card: "gap-3",
  group: "flex min-w-0 flex-col gap-1.5",
  term: "text-xs text-muted-foreground",
  tags: "flex flex-wrap gap-1",
  versionBadge: "border-border",
  links: "-mx-2 flex flex-col",
  link: "w-full justify-start",
  creator:
    "flex items-center gap-2 text-sm [&_svg]:size-4 [&_svg]:text-muted-foreground",
  details: "flex flex-col gap-2 text-sm",
  detail: "flex items-baseline justify-between gap-3",
  value: "min-w-0 truncate text-right",
  inlineLink:
    "cursor-pointer truncate underline underline-offset-3 outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50",
} as const;
