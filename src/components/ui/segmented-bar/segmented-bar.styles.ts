export const segmentedBarStyles = {
  root: "flex w-full min-w-0 max-w-full flex-col gap-3",
  track:
    "relative flex h-5 w-full min-w-0 max-w-full items-center overflow-hidden",
  chart: "flex w-full min-w-0 max-w-full items-center",
  bar: "relative w-full min-w-0 max-w-full overflow-hidden rounded-full",
  barBackground: "fill-muted",
  segment:
    "transition-opacity duration-200 ease-out motion-reduce:transition-none",
  segmentDimmed: "opacity-25",
  segmentVisible: "opacity-100",
  highlight:
    "pointer-events-none absolute top-1/2 left-0 w-full min-w-0 max-w-full -translate-y-1/2 overflow-hidden rounded-full transition-[height,opacity] duration-200 ease-out motion-reduce:transition-none",
  highlightVisible: "opacity-100",
  highlightHidden: "opacity-0",
  loading:
    "absolute top-1/2 left-0 h-3 -translate-y-1/2 motion-reduce:[&_[data-slot=progress-indicator]]:animate-none",
  legend: "text-xs",
  legendCompact: "grid grid-cols-3 gap-x-4 gap-y-2",
  legendRegular: "flex flex-col gap-1",
  legendItemCompact: "min-w-0",
  legendButton: "h-auto w-full justify-between text-xs",
  legendButtonCompact: "gap-2 px-0 py-0.5",
  legendButtonRegular: "gap-3 px-1 py-1",
  legendButtonActive: "text-foreground",
  legendButtonInactive: "text-muted-foreground",
  legendLabel: "flex min-w-0 items-center gap-2",
  legendMarker: "size-2 shrink-0 rounded-sm",
  legendValue: "shrink-0 font-mono tabular-nums",
  legendSpinner: "size-3",
  colors: {
    primary: { fill: "fill-primary", marker: "bg-primary" },
    warning: { fill: "fill-warning", marker: "bg-warning" },
    chart1: { fill: "fill-chart-1", marker: "bg-chart-1" },
    chart2: { fill: "fill-chart-2", marker: "bg-chart-2" },
    chart3: { fill: "fill-chart-3", marker: "bg-chart-3" },
    violet: { fill: "fill-accent-violet", marker: "bg-accent-violet" },
  },
} as const;
