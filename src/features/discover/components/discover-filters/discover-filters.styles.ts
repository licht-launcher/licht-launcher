export const discoverFiltersStyles = {
  root: "flex w-56 shrink-0 flex-col rounded-lg border border-border",
  scrollArea: "min-h-0 flex-1",
  content: "flex flex-col gap-5 p-3",
  group: "flex flex-col gap-2",
  heading:
    "flex items-center gap-1.5 text-xs font-medium text-muted-foreground",
  headingIcon: "size-3.5",
  trigger:
    "w-full *:data-[slot=select-value]:flex-1 *:data-[slot=select-value]:text-left",
  categories: "flex flex-col gap-0.5",
  category:
    "w-full justify-start text-muted-foreground aria-pressed:bg-muted aria-pressed:text-foreground",
  clear: "self-start",
} as const;
