export const updateCheckStyles = {
  toast:
    "pointer-events-auto flex w-full max-w-sm items-start gap-3 rounded-xl border border-border bg-popover p-3 text-popover-foreground",
  spinner: "size-4 shrink-0 animate-spin",
  icon: "size-4 shrink-0",
  toastText: "flex min-w-0 flex-1 flex-col gap-0.5",
  toastTitle: "text-sm font-medium",
  toastDescription: "text-sm text-muted-foreground",
  prompt:
    "pointer-events-auto flex w-full max-w-sm flex-col gap-3 rounded-xl border border-border bg-popover p-3 text-popover-foreground",
  promptText: "flex flex-col gap-0.5",
  promptActions: "flex items-center justify-end gap-2",
} as const;
