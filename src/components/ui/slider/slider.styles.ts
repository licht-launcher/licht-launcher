export const sliderStyles = {
  root: "relative flex w-full touch-none items-center select-none data-disabled:opacity-50",
  track: "relative h-1.5 w-full grow overflow-hidden rounded-full bg-muted",
  range: "absolute h-full bg-primary",
  thumb:
    "relative z-20 block size-4 rounded-full border border-primary bg-white outline-none focus-visible:ring-3 focus-visible:ring-white/80",
} as const;
