export const comboboxStyles = {
  root: "relative",
  input: "pr-8",
  chevron:
    "pointer-events-none absolute top-1/2 right-2.5 size-4 -translate-y-1/2 text-muted-foreground transition-transform duration-200",
  chevronOpen: "rotate-180",
  content: "w-(--radix-popover-trigger-width) p-1",
  viewport:
    "max-h-[min(16rem,var(--radix-popper-available-height))] overflow-y-auto text-sm",
  empty: "px-2 py-2 text-muted-foreground",
  listbox: "relative w-full",
  option:
    "absolute inset-x-0 top-0 flex cursor-pointer items-center justify-between rounded-md px-2 transition-colors",
  optionActive: "bg-muted",
  optionSelected: "font-medium",
  checkIcon: "size-4 text-primary",
} as const;
