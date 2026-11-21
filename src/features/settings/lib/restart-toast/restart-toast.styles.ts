export const titleBarRestartToastStyles = {
  root: "group/toast pointer-events-auto relative z-100 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-3 overflow-hidden rounded-xl border border-border bg-popover px-3 pt-3 pb-3.5 text-popover-foreground shadow-none",
  header: "flex items-start gap-3",
  iconWell:
    "flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground",
  icon: "size-4",
  text: "min-w-0 flex-1",
  title: "font-brand text-sm font-medium",
  message: "text-sm text-muted-foreground",
  actions: "flex items-center gap-1.5",
  action: "min-w-0 flex-1 cursor-pointer",
  progressBar:
    "toast-progress-bar absolute inset-x-0 bottom-0 h-0.5 bg-primary",
} as const;
