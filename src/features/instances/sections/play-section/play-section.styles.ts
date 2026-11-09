export const playSectionStyles = {
  metaTrigger:
    "inline-flex min-w-0 cursor-default items-center gap-1 rounded-sm text-sm text-muted-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring",
  metaIcon: "size-3.5 shrink-0",
  muted: "text-sm text-muted-foreground",
  root: "flex flex-col gap-3",
  header: "flex min-w-0 flex-wrap items-center justify-between gap-3",
  identity: "flex min-w-0 items-center gap-3",
  identityText: "flex min-w-0 flex-col gap-0.5",
  title: "text-lg font-medium",
  meta: "flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5",
  metaValue: "truncate",
  actions: "flex shrink-0 flex-wrap items-center gap-2",
  busyButton: "animate-pulse disabled:opacity-100",
  download: "flex w-full min-w-0 flex-col gap-1",
  downloadLabel: "text-xs text-muted-foreground",
  downloadTitle: "font-medium",
  downloadDetail: "ml-2",
} as const;
