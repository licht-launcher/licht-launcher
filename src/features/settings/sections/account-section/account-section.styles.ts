export const accountSectionStyles = {
  row: "flex items-center justify-between gap-3",
  rowText: "flex min-w-0 flex-col gap-0.5",
  rowLabel: "text-sm font-normal",
  rowDescription: "text-sm text-muted-foreground",
  spinner: "animate-spin",
  error: "text-sm text-destructive",
  profiles: "flex flex-col gap-2",
  profile:
    "flex items-center justify-between gap-2 rounded-md border border-border px-3 py-2",
  profileText: "min-w-0",
  profileName: "truncate text-sm font-normal",
  profileId: "truncate font-mono text-xs text-muted-foreground",
  profileActions: "flex shrink-0 gap-2",
  profileStatus: "text-xs text-muted-foreground",
} as const;
