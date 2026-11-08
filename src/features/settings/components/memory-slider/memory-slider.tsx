import type { CSSProperties } from "react";

import { Slider } from "@/components/ui/slider";
import { formatMemoryPreset, memoryPresetOptions } from "@/lib/memory-presets";
import { cn } from "@/lib/utils";

import { memorySliderStyles } from "./memory-slider.styles";

/** Half of `size-4` thumb — matches Radix in-bounds thumb centers. */
const THUMB_HALF_PX = 8;

function markTranslateClass(index: number, count: number): string {
  if (count <= 1 || index === 0) {
    return memorySliderStyles.markAlignStart;
  }
  if (index === count - 1) {
    return memorySliderStyles.markAlignEnd;
  }
  return memorySliderStyles.markAlignCenter;
}

function labelLeftStyle(
  index: number,
  count: number,
  percent: number,
): CSSProperties {
  if (count <= 1 || index === 0 || index === count - 1) {
    return { left: `${percent}%` };
  }
  const inset = THUMB_HALF_PX - (percent / 100) * (THUMB_HALF_PX * 2);
  return { left: `calc(${percent}% + ${inset}px)` };
}

function tickLeftStyle(percent: number): CSSProperties {
  const inset = THUMB_HALF_PX - (percent / 100) * (THUMB_HALF_PX * 2);
  return { left: `calc(${percent}% + ${inset}px)` };
}

export function MemorySlider({
  valueMb,
  minimumMb,
  maximumMb,
  recommendedMb,
  totalMb,
  stepMb,
  onChange,
}: {
  valueMb: number;
  minimumMb: number;
  maximumMb: number;
  recommendedMb: number;
  totalMb: number;
  stepMb: number;
  onChange: (maxMemoryMb: number) => void;
}) {
  const options = memoryPresetOptions({
    minimumMb,
    maximumMb,
    recommendedMb,
    totalMb,
  });
  const ceiling = Math.max(minimumMb, Math.min(maximumMb, totalMb));
  const bounded = (value: number) =>
    Math.min(ceiling, Math.max(minimumMb, value));
  const percentage = (value: number) =>
    ceiling === minimumMb
      ? 0
      : ((value - minimumMb) / (ceiling - minimumMb)) * 100;

  return (
    <div className={memorySliderStyles.root}>
      <Slider
        aria-label="Memory"
        className={memorySliderStyles.slider}
        min={minimumMb}
        max={ceiling}
        step={1}
        value={[bounded(valueMb)]}
        aria-valuetext={`${valueMb} MB`}
        onKeyDownCapture={(event) => {
          let next = valueMb;
          const increment = stepMb * (event.shiftKey ? 10 : 1);
          switch (event.key) {
            case "Home":
              next = minimumMb;
              break;
            case "End":
              next = ceiling;
              break;
            case "ArrowRight":
            case "ArrowUp":
              next += increment;
              break;
            case "ArrowLeft":
            case "ArrowDown":
              next -= increment;
              break;
            case "PageUp":
              next += stepMb * 10;
              break;
            case "PageDown":
              next -= stepMb * 10;
              break;
            default:
              return;
          }
          event.preventDefault();
          onChange(bounded(next));
        }}
        onValueChange={(value) => {
          const next = value[0];
          if (next === undefined) return;
          // Snap dragging separately so typed values and the exact RAM endpoint stay representable.
          onChange(
            next === ceiling
              ? ceiling
              : bounded(
                  minimumMb + Math.round((next - minimumMb) / stepMb) * stepMb,
                ),
          );
        }}
      >
        <div aria-hidden className={memorySliderStyles.ticks}>
          {options.map((mb) => {
            const percent = percentage(mb);
            const selected = mb === valueMb;
            return (
              <span
                key={mb}
                className={
                  selected
                    ? memorySliderStyles.tickSelected
                    : memorySliderStyles.tick
                }
                style={tickLeftStyle(percent)}
              />
            );
          })}
        </div>
      </Slider>
      <div className={memorySliderStyles.marks}>
        {options.map((mb, optionIndex) => {
          const selected = mb === valueMb;
          const percent = percentage(mb);
          const isEnd =
            options.length > 1 &&
            (optionIndex === 0 || optionIndex === options.length - 1);
          return (
            <button
              key={mb}
              type="button"
              className={cn(
                memorySliderStyles.mark,
                markTranslateClass(optionIndex, options.length),
                selected
                  ? memorySliderStyles.markSelected
                  : memorySliderStyles.markIdle,
                isEnd
                  ? memorySliderStyles.markEnd
                  : memorySliderStyles.markMiddle,
              )}
              style={labelLeftStyle(optionIndex, options.length, percent)}
              aria-label={formatMemoryPreset(mb)}
              aria-pressed={selected}
              onClick={() => {
                onChange(mb);
              }}
            >
              {formatMemoryPreset(mb)}
            </button>
          );
        })}
      </div>
    </div>
  );
}
