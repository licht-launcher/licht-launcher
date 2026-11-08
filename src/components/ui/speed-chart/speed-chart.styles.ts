export const speedChartStyles = {
  root: "flex min-w-0 flex-col rounded-lg border border-border/60 bg-black/20 p-4",
  title: "flex items-center gap-2 text-sm font-medium",
  titleIcon: "size-4 text-muted-foreground",
  legend: "mt-2 flex flex-wrap gap-x-6 gap-y-2 text-sm",
  legendItem: "flex items-center gap-2",
  legendSwatch: "size-3",
  legendLabel: "text-xs font-medium text-muted-foreground",
  legendValue: "font-mono text-sm font-medium tabular-nums",
  legendValueBars: "text-foreground",
  legendValueLine: "text-muted-foreground",
  chart: "mt-2 h-32 min-h-32 w-full flex-1",
  grid: "text-border",
  line: "opacity-50",
  colors: {
    violet: "text-accent-violet",
    cool: "text-accent-cool",
  },
} as const;
