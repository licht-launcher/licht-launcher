export const modpackContentDialogStyles = {
  content: "flex h-4/5 flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl",
  header: "flex-row items-center gap-3 border-b border-border/60 px-6 py-4",
  icon: "size-10 shrink-0 rounded-md bg-muted object-cover",
  iconFallback:
    "flex size-10 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:size-5",
  title: "min-w-0 flex-1 truncate text-lg font-semibold",
  description: "sr-only",
  toolbar: "flex flex-col gap-3 px-6 py-4",
  filters: "m-0 flex min-w-0 flex-wrap items-center gap-2 border-0 p-0",
  legend: "sr-only",
  list: "flex min-h-0 flex-1 flex-col border-t border-border/60",
  headTable: "w-full shrink-0 table-fixed text-sm",
  footer:
    "flex flex-wrap items-center gap-x-5 gap-y-1 border-t border-border/60 px-6 py-3 text-sm text-muted-foreground",
  count: "flex items-center gap-2 tabular-nums",
} as const;
