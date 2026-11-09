export const editInstanceVersionDialogStyles = {
  content: "sm:max-w-md",
  body: "flex flex-col gap-4",
  field: "flex flex-col gap-1",
  label: "text-sm font-medium",
  value: "flex h-8 items-center rounded-lg bg-muted/40 px-2.5 text-sm",
  loading: "flex h-8 items-center gap-2 text-sm text-muted-foreground",
  loadingIcon: "size-3.5",
  errorBlock: "flex flex-col gap-2",
  message: "text-sm text-muted-foreground",
  retryButton: "self-start",
  selectTrigger: "w-full font-normal",
} as const;
