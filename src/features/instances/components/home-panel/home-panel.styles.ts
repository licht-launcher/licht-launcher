export const homePanelStyles = {
  root: "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden p-4",
  header: "shrink-0",
  tabs: "mt-3 flex min-h-0 flex-1 flex-col gap-2",
  toolbar: "flex shrink-0 flex-col gap-2",
  tabsList: "w-fit shrink-0",
  tabsTrigger: "cursor-pointer gap-1.5 px-3",
  tabIcon: "size-4 shrink-0",
  tabActions: "flex min-w-0 items-center gap-2",
  searchInput: "min-w-0 flex-1 font-medium",
  refreshing: "animate-spin",
  tabContent: "mt-0 flex min-h-0 flex-1 flex-col outline-none",
} as const;
