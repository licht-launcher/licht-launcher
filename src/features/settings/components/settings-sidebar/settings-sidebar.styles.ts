export const settingsSidebarStyles = {
  navButton:
    "h-auto flex items-center justify-start gap-2 px-2.5 py-1.75 font-medium",
  navIcon: "size-4 shrink-0",
  navLabel: "min-w-0 truncate",
  root: "flex w-48 shrink-0 flex-col gap-4 border-r border-border px-3 py-4",
  title: "sr-only",
  nav: "flex flex-col gap-0",
  footer: "mt-auto flex min-w-0 flex-col gap-2",
  footerLinks: "flex flex-col",
  versionButton:
    "h-auto w-full justify-start px-2.5 py-1 font-mono text-xs font-normal text-muted-foreground transition-none hover:bg-transparent hover:text-muted-foreground dark:hover:bg-transparent",
  versionLabel: "truncate",
} as const;
