export const memorySliderStyles = {
  root: "flex w-full flex-col gap-3",
  slider: "w-full",
  ticks: "pointer-events-none absolute inset-x-0 top-1/2 z-10 h-0",
  tickSelected:
    "absolute top-1/2 h-2.5 w-px -translate-x-1/2 -translate-y-1/2 bg-primary-foreground/80",
  tick: "absolute top-1/2 h-2.5 w-px -translate-x-1/2 -translate-y-1/2 bg-muted-foreground/50",
  marks: "relative h-8 w-full",
  mark: "absolute top-0 cursor-pointer rounded-md text-[0.8rem] whitespace-nowrap",
  markSelected: "bg-primary/15 font-medium text-foreground",
  markIdle: "text-muted-foreground hover:bg-muted hover:text-foreground",
  markEnd: "px-2 py-1",
  markMiddle: "px-2.5 py-1",
  markAlignStart: "translate-x-0",
  markAlignEnd: "-translate-x-full",
  markAlignCenter: "-translate-x-1/2",
} as const;
