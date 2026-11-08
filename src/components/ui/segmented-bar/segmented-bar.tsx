import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

import { segmentedBarStyles } from "./segmented-bar.styles";

const colors = segmentedBarStyles.colors;

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
  compact = false,
}: {
  label: string;
  segments: readonly Segment[];
  loading?: boolean;
  compact?: boolean;
}) {
  const [hovered, setHovered] = useState<string | null>(null);

  const active = loading ? null : hovered;
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
      className={segmentedBarStyles.root}
      onMouseOver={(event) => setHovered(segmentFrom(event.target))}
      onMouseLeave={() => setHovered(null)}
      onFocus={(event) => setHovered(segmentFrom(event.target))}
      onBlur={() => setHovered(null)}
    >
      <div className={segmentedBarStyles.track}>
        <div
          role="img"
          aria-label={`${label}. ${summary}`}
          aria-hidden={loading}
          className={segmentedBarStyles.chart}
        >
          <svg
            aria-hidden
            aria-label={`${label} bar`}
            viewBox="0 0 1000 12"
            preserveAspectRatio="none"
            width="100%"
            height="12"
            className={segmentedBarStyles.bar}
          >
            <rect
              width="1000"
              height="12"
              className={segmentedBarStyles.barBackground}
            />
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
                  segmentedBarStyles.segment,
                  active !== null && active !== block.label
                    ? segmentedBarStyles.segmentDimmed
                    : segmentedBarStyles.segmentVisible,
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
              segmentedBarStyles.highlight,
              highlighted
                ? segmentedBarStyles.highlightVisible
                : segmentedBarStyles.highlightHidden,
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
        </div>
        {loading && (
          <Progress
            aria-label={`Loading ${label.toLowerCase()}`}
            className={segmentedBarStyles.loading}
          />
        )}
      </div>
      <ul
        aria-label={`${label} breakdown`}
        className={cn(
          segmentedBarStyles.legend,
          compact
            ? segmentedBarStyles.legendCompact
            : segmentedBarStyles.legendRegular,
        )}
      >
        {segments.map((segment) => (
          <li
            key={segment.label}
            className={
              compact ? segmentedBarStyles.legendItemCompact : undefined
            }
          >
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
                segmentedBarStyles.legendButton,
                compact
                  ? segmentedBarStyles.legendButtonCompact
                  : segmentedBarStyles.legendButtonRegular,
                active === segment.label
                  ? segmentedBarStyles.legendButtonActive
                  : segmentedBarStyles.legendButtonInactive,
              )}
            >
              <span className={segmentedBarStyles.legendLabel}>
                <span
                  aria-hidden
                  className={`${segmentedBarStyles.legendMarker} ${colors[segment.color].marker}`}
                />
                {segment.label}
              </span>
              <span className={segmentedBarStyles.legendValue}>
                {loading ? (
                  <Spinner
                    aria-label={`Loading ${segment.label}`}
                    className={segmentedBarStyles.legendSpinner}
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
