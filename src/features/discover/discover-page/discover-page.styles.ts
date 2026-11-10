export const discoverPageStyles = {
  root: "flex min-h-0 flex-1 flex-col gap-4 overflow-hidden p-4",
  header: "flex shrink-0 flex-wrap items-center justify-between gap-3",
  title: "flex items-center gap-2 text-lg font-semibold",
  titleIcon: "size-5 text-muted-foreground",
  controls: "flex w-full max-w-lg items-center gap-2",
  search: "min-w-0 flex-1",
  body: "flex min-h-0 flex-1 gap-4",
  main: "flex min-h-0 min-w-0 flex-1 flex-col gap-4",
  results:
    "flex min-h-0 flex-1 flex-col rounded-lg border border-border bg-background",
  scrollArea: "min-h-0 flex-1",
  list: "flex flex-col gap-0.5 p-1.5",
  loading: "flex flex-1 items-center justify-center",
  footer: "shrink-0",
} as const;
