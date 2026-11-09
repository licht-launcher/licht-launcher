export const homeSidebarStyles = {
  sortLabel:
    "flex items-center gap-1.5 font-brand text-xs font-medium leading-none",
  sortIcon: "size-3.5 shrink-0",
  root: "flex h-full w-full min-w-0 flex-col overflow-hidden",
  body: "flex min-h-0 flex-1 flex-col px-2 py-2",
  nav: "flex min-h-0 flex-1 flex-col gap-2",
  section: "flex min-h-0 flex-1 flex-col",
  sectionHeader: "ml-px flex items-center gap-2 border border-transparent px-1",
  sectionTitle:
    "min-w-0 flex-1 font-brand text-xs font-medium leading-none text-muted-foreground",
  sortTrigger:
    "h-auto w-auto max-w-40 shrink-0 gap-1 border-0 bg-transparent px-0 py-0 font-brand text-xs font-medium leading-none text-muted-foreground shadow-none dark:bg-transparent dark:hover:bg-transparent [&_svg:not([class*='size-'])]:size-3.5",
  sortContent: "z-100 font-brand text-xs font-medium",
  sortItem: "text-xs",
  empty: "border border-transparent px-1 text-sm text-muted-foreground",
  scrollArea: "min-h-0 flex-1",
  list: "flex w-full min-w-0 flex-col px-px",
  listItem: "min-w-0 max-w-full",
  instanceButton:
    "grid w-full min-w-0 max-w-full grid-cols-[auto_minmax(0,1fr)_auto] gap-2 overflow-hidden px-1 text-[0.8rem]",
  instanceButtonSelected:
    "border-foreground/10 bg-secondary/35 hover:border-foreground/20 hover:bg-secondary/50",
  instanceButtonIdle: "text-muted-foreground/70",
  instanceName:
    "min-w-0 overflow-hidden text-left text-ellipsis whitespace-nowrap",
  instanceStatus:
    "inline-flex min-w-0 items-center gap-1.5 overflow-hidden text-ellipsis whitespace-nowrap text-xs font-medium text-success",
  instanceSpinner: "size-3.5 shrink-0 animate-spin",
  runningDot: "relative flex size-2 shrink-0",
  runningPing:
    "absolute inline-flex size-full animate-ping rounded-full bg-success opacity-75",
  runningCore: "relative inline-flex size-2 rounded-full bg-success",
} as const;
