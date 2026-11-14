// Shared by Loader and Category: a vertical list of icon buttons.
const optionList = "flex flex-col gap-0.5 pl-2";
const option =
  "w-full justify-start text-muted-foreground [&_svg]:-translate-y-px";

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
  // Indented so each icon lines up with the group title, past its chevron.
  loaders: optionList,
  categories: optionList,
  loaderIcon: "size-3.5",
  // The font sits its glyphs high in the line box; lifting the icon a pixel
  // centers it on the label.
  loader: option,
  // Chosen categories keep the neutral background and gain a border.
  category: `${option} aria-pressed:border-border aria-pressed:bg-secondary aria-pressed:text-foreground aria-pressed:hover:border-border aria-pressed:hover:bg-secondary aria-pressed:hover:text-foreground dark:aria-pressed:hover:bg-secondary`,
} as const;
