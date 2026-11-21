export const startupScreenStyles = {
  root: "flex h-full min-h-screen flex-col bg-sidebar text-sidebar-foreground",
  brand:
    "flex items-center px-3.5 font-brand text-xs font-medium tracking-widest text-muted-foreground uppercase select-none",
  content: "flex min-h-0 flex-1 items-center justify-center p-6 text-center",
  progress: "flex w-full max-w-md flex-col gap-2.5",
  detail: "text-xs text-muted-foreground",
  message: "text-sm font-medium",
  warning: "text-sm text-warning",
  warningLabel: "text-xs font-medium text-warning",
  error: "text-sm text-destructive",
} as const;
