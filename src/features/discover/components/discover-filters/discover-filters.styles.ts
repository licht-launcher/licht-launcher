export const discoverFiltersStyles = {
  root: "flex w-56 shrink-0 flex-col rounded-lg border border-border bg-background",
  scrollArea: "min-h-0 flex-1",
  content: "flex flex-col gap-5 p-3",
  group: "flex flex-col gap-2",
  heading: "text-xs font-medium text-muted-foreground",
  trigger: "w-full",
  categories: "flex flex-col gap-0.5",
  category:
    "w-full justify-start text-muted-foreground aria-pressed:bg-muted aria-pressed:text-foreground",
  clear: "self-start",
} as const;
