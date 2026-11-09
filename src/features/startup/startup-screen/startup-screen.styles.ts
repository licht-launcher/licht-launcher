export const startupScreenStyles = {
  scrollArea: "h-full",
  root: "flex min-h-screen flex-col items-center justify-center gap-6 bg-sidebar p-6 text-center text-sidebar-foreground",
  logoWrapper: "flex w-full justify-center",
  logo: "pointer-events-none size-24 object-contain",
  progress: "flex w-full max-w-xs flex-col gap-2",
  detail: "text-xs text-muted-foreground",
  message: "text-sm",
  warning: "text-sm text-warning",
  warningLabel: "text-xs font-medium text-warning",
  error: "text-sm text-destructive",
} as const;
