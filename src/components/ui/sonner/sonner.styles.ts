export const toasterStyles = {
  root: "toaster group z-100",
  icon: "size-4",
  loadingIcon: "size-4 animate-spin",
  toast: "cn-toast pointer-events-auto",
  closeButton:
    "absolute right-2 top-2 inline-flex size-5 items-center justify-center rounded-sm text-muted-foreground opacity-70 transition-opacity hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
} as const;
