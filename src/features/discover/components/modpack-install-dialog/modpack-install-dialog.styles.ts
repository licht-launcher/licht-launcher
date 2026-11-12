export const modpackInstallDialogStyles = {
  content: "sm:max-w-md",
  form: "flex flex-col gap-5",
  status: "flex items-center gap-2 text-sm text-muted-foreground",
  error: "text-sm text-destructive",
  // Override the trigger's flex value so long labels ellipsize inside w-full.
  select:
    "w-full min-w-0 font-normal *:data-[slot=select-value]:block *:data-[slot=select-value]:min-w-0 *:data-[slot=select-value]:truncate",
} as const;
