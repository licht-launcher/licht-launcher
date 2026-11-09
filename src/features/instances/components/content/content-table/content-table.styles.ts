export const contentTableStyles = {
  scrollArea: "min-h-0 flex-1",
  tableWrapper: "min-w-[48rem]",
  table: "w-full table-fixed text-sm",
  empty:
    "flex min-h-56 flex-col items-center justify-center gap-3 px-4 text-center",
  emptyIconWell:
    "flex size-12 items-center justify-center rounded-full bg-muted text-muted-foreground",
  emptyIcon: "size-6",
  emptyText:
    "font-brand text-sm font-medium leading-normal text-muted-foreground",
  emptyCell: "border-0 p-0",
} as const;
