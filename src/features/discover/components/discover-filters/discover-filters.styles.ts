export const discoverFiltersStyles = {
  root: "flex w-56 shrink-0 flex-col",
  scrollArea: "min-h-0 flex-1",
  content: "flex flex-col gap-4 py-1 pr-3 pl-1",
  group: "flex flex-col gap-2",
  heading:
    "group/heading flex w-full cursor-pointer items-center gap-1.5 rounded-md text-xs font-medium text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50",
  chevron:
    "size-3.5 transition-transform group-data-[state=closed]/heading:-rotate-90",
  groupContent: "flex flex-col",
  trigger:
    "w-full *:data-[slot=select-value]:flex-1 *:data-[slot=select-value]:text-left",
  loaderIcon: "size-4",
  // Indented so each icon lines up with the group title, past its chevron.
  categories: "flex flex-col gap-0.5 pl-2",
  // The font sits its glyphs high in the line box; lifting the icon a pixel
  // centers it on the label.
  category:
    "w-full justify-start text-muted-foreground [&_svg]:-translate-y-px",
} as const;
