export const launcherNavigationStyles = {
  root: "flex shrink-0 items-center gap-1 px-2",
  brandTrigger:
    "mr-1 h-7 shrink-0 gap-1 px-1.5 font-brand text-xs font-medium tracking-widest text-muted-foreground uppercase select-none hover:bg-transparent dark:hover:bg-transparent aria-expanded:bg-transparent",
  brandChevron: "size-3",
  menu: "font-brand",
  history: "flex shrink-0 items-center gap-0.5",
  historyButton: "text-muted-foreground",
  nav: "ml-2 flex shrink-0 items-center gap-1",
  navButton: "h-7 px-2 text-xs text-muted-foreground",
  navButtonCurrent:
    "aria-[current=page]:border-foreground/20 aria-[current=page]:text-foreground",
  navIcon: "size-3.5",
} as const;
