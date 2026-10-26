import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

const colors = {
  primary: { fill: "fill-primary", marker: "bg-primary" },
  warning: { fill: "fill-warning", marker: "bg-warning" },
  chart1: { fill: "fill-chart-1", marker: "bg-chart-1" },
  chart2: { fill: "fill-chart-2", marker: "bg-chart-2" },
  chart3: { fill: "fill-chart-3", marker: "bg-chart-3" },
  violet: { fill: "fill-accent-violet", marker: "bg-accent-violet" },
};

export type SegmentColor = keyof typeof colors;
type Segment = {
  label: string;
  value: number;
  valueLabel: string;
  color: SegmentColor;
};

function segmentFrom(target: EventTarget): string | null {
  return target instanceof Element
    ? (target.closest("[data-segment]")?.getAttribute("data-segment") ?? null)
    : null;
}

export function SegmentedBar({
  label,
  segments,
  loading = false,
}: {
  label: string;
  segments: readonly Segment[];
  loading?: boolean;
}) {
  const [hovered, setHovered] = useState<string | null>(null);
  const [focused, setFocused] = useState<string | null>(null);
  const active = loading ? null : (hovered ?? focused);
  const total = segments.reduce(
    (sum, segment) => sum + Math.max(0, segment.value),
    0,
  );
  let offset = 0;
  const blocks = (loading ? [] : segments)
    .filter((segment) => segment.value > 0)
    .map((segment, index, nonempty) => {
      const width =
        index === nonempty.length - 1
          ? Math.max(0, 1000 - offset)
          : (segment.value / total) * 1000;
      const block = { ...segment, x: offset, width };
      offset += width;
      return block;
    });
  const highlighted = blocks.find((block) => block.label === active);
  const summary = segments
    .map(
      (segment) =>
        `${segment.label}: ${loading ? "Loading" : segment.valueLabel}`,
    )
    .join("; ");

  return (
    <fieldset
      aria-label={`${label} chart`}
      aria-busy={loading}
      className="flex w-full min-w-0 max-w-full flex-col gap-3"
      onMouseOver={(event) => setHovered(segmentFrom(event.target))}
      onMouseLeave={() => setHovered(null)}
      onFocus={(event) => setFocused(segmentFrom(event.target))}
      onBlur={() => setFocused(null)}
    >
      <div
        role="img"
        aria-label={`${label}. ${summary}`}
        className="relative flex h-5 w-full min-w-0 max-w-full items-center overflow-hidden"
      >
        <svg
          aria-hidden
          aria-label={`${label} bar`}
          viewBox="0 0 1000 12"
          preserveAspectRatio="none"
          width="100%"
          height="12"
          className="relative w-full min-w-0 max-w-full overflow-hidden rounded-full"
        >
          <rect width="1000" height="12" className="fill-muted" />
          {blocks.map((block) => (
            <rect
              key={block.label}
              aria-label={`${block.label} segment`}
              data-segment={block.label}
              x={block.x}
              y="0"
              width={block.width}
              height="12"
              className={cn(
                colors[block.color].fill,
                "transition-opacity duration-200 ease-out motion-reduce:transition-none",
                active !== null && active !== block.label
                  ? "opacity-25"
                  : "opacity-100",
              )}
            />
          ))}
        </svg>
        <svg
          aria-hidden
          aria-label={`${label} highlight`}
          viewBox="0 0 1000 16"
          preserveAspectRatio="none"
          width="100%"
          height={highlighted ? 16 : 12}
          className={cn(
            "pointer-events-none absolute top-1/2 left-0 w-full min-w-0 max-w-full -translate-y-1/2 overflow-hidden rounded-full transition-[height,opacity] duration-200 ease-out motion-reduce:transition-none",
            highlighted ? "opacity-100" : "opacity-0",
          )}
        >
          {highlighted && (
            <rect
              x={highlighted.x}
              y="0"
              width={highlighted.width}
              height="16"
              className={colors[highlighted.color].fill}
            />
          )}
        </svg>
        {loading && (
          <Spinner
            aria-label={`Loading ${label.toLowerCase()}`}
            className="absolute top-1/2 left-1/2 size-3 -translate-x-1/2 -translate-y-1/2 text-muted-foreground"
          />
        )}
      </div>
      <ul
        aria-label={`${label} breakdown`}
        className="flex flex-col gap-1 text-xs"
      >
        {segments.map((segment) => (
          <li key={segment.label}>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              aria-label={
                loading
                  ? `Loading ${segment.label}`
                  : `Highlight ${segment.label}: ${segment.valueLabel}`
              }
              disabled={loading}
              data-segment={segment.label}
              className={cn(
                "h-auto w-full justify-between gap-3 px-1 py-1 text-xs",
                active === segment.label
                  ? "text-foreground"
                  : "text-muted-foreground",
              )}
              onClick={() => setFocused(segment.label)}
            >
              <span className="flex min-w-0 items-center gap-2 text-muted-foreground">
                <span
                  aria-hidden
                  className={`size-2 shrink-0 rounded-sm ${colors[segment.color].marker}`}
                />
                {segment.label}
              </span>
              <span className="shrink-0 font-mono tabular-nums">
                {loading ? (
                  <Spinner
                    aria-label={`Loading ${segment.label}`}
                    className="size-3"
                  />
                ) : (
                  segment.valueLabel
                )}
              </span>
            </Button>
          </li>
        ))}
      </ul>
    </fieldset>
  );
}
