export const changeModpackVersionDialogStyles = {
  content: "sm:max-w-md",
  body: "flex flex-col gap-2",
  label: "text-sm font-medium",
  loading: "flex h-8 items-center gap-2 text-sm text-muted-foreground",
  loadingIcon: "size-3.5",
  errorBlock: "flex flex-col gap-2",
  message: "text-sm text-muted-foreground",
  retryButton: "self-start",
  selectTrigger: "w-full font-normal",
  installed: "text-muted-foreground",
} as const;
