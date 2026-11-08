export const choiceCardStyles = {
  root: [
    "flex h-auto w-full cursor-pointer flex-col items-start gap-3 rounded-lg border py-1 pl-1 pr-2.5 text-left text-foreground outline-none select-none",
    "transition-[background-color,border-color,box-shadow] duration-200 ease-out",
    "focus-visible:ring-3 focus-visible:ring-ring/50",
    "disabled:pointer-events-none disabled:opacity-50",
    "border-border/60 ring-3",
  ],
  rootSelected: "bg-muted ring-primary",
  rootUnselected: "ring-primary/0",
  body: "flex flex-col items-start w-full gap-2",
  header: "flex items-center gap-2",
  icon: "flex border size-7 p-1.5 shrink-0 items-center justify-center overflow-hidden rounded-md transition-colors duration-200 ease-out",
  iconNeutral: "bg-black/20 text-muted-foreground",
  iconTones: {
    instance: "bg-instance/20 text-instance",
    warning: "bg-warning/20 text-warning",
    cool: "bg-accent-cool/20 text-accent-cool",
    file: "bg-accent-file/20 text-accent-file",
  },
  title: "block text-sm",
} as const;
