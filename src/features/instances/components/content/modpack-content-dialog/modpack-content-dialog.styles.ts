export const modpackContentDialogStyles = {
  content: "flex h-4/5 flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl",
  header: "flex-row items-center gap-3 border-b border-border/60 px-6 py-4",
  icon: "size-9 shrink-0 rounded-md bg-muted object-cover",
  iconFallback:
    "flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:size-5",
  title: "min-w-0 flex-1 truncate text-base font-semibold",
  description: "sr-only",
  summary:
    "flex flex-wrap items-center gap-x-5 gap-y-1 px-6 pt-4 text-sm font-medium",
  count: "flex items-center gap-2 tabular-nums",
  toolbar: "flex items-center gap-2 px-6 py-4",
  searchInput: "min-w-0 flex-1 font-medium",
  /** The rows pad their cells by 12px, so this inset lines them up with the 24px page margin. */
  list: "flex min-h-0 flex-1 flex-col border-t border-border/60 px-3",
  headTable: "w-full shrink-0 table-fixed text-sm",
} as const;
