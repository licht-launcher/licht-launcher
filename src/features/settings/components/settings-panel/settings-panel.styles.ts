export const settingsPanelStyles = {
  root: "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
  scrollArea: "min-h-0 flex-1",
  content: "flex flex-col gap-6 px-8 pt-8 pr-12 pb-8",
  header: "flex flex-col gap-0.5",
  title: "text-lg font-medium",
  description: "text-sm leading-snug text-muted-foreground",
  sections: "flex flex-col gap-8",
} as const;
