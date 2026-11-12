export const modpackVersionsStyles = {
  state: "flex flex-1 items-center justify-center py-10",
  scrollArea: "min-h-0 flex-1",
  /** A grid table, so virtualized rows can be placed absolutely. */
  table: "grid w-full text-sm",
  head: "sticky top-0 z-10 grid border-b border-border bg-background",
  /** Shared by the head and every row, so the columns line up. */
  columns:
    "grid w-full grid-cols-[minmax(0,2fr)_minmax(0,1.2fr)_minmax(0,1.4fr)_6.5rem_5.5rem_6rem] items-center gap-3 px-3",
  headCell: "py-2 text-left text-xs font-medium text-muted-foreground",
  body: "relative grid",
  row: "absolute top-0 left-0 border-b border-border/60 py-2.5",
  nameCell: "flex min-w-0 items-center gap-2",
  name: "truncate font-medium",
  muted: "truncate text-muted-foreground",
  loaders: "flex min-w-0 flex-wrap gap-1",
  number: "text-muted-foreground tabular-nums",
  action: "flex justify-end",
  srOnly: "sr-only",
  /** Releases read as safe, betas as caution, alphas as risky. */
  typeTones: {
    release: "bg-success/15 text-success",
    beta: "bg-warning/15 text-warning-foreground dark:text-warning",
    alpha: "bg-destructive/15 text-destructive",
  },
} as const;
