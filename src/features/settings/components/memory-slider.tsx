import type { CSSProperties } from "react";

import { Slider } from "@/components/ui/slider";
import { formatMemoryPreset, memoryPresetOptions } from "@/lib/memory-presets";

/** Half of `size-4` thumb — matches Radix in-bounds thumb centers. */
const THUMB_HALF_PX = 8;

function markTranslateClass(index: number, count: number): string {
  if (count <= 1 || index === 0) {
    return "translate-x-0";
  }
  if (index === count - 1) {
    return "-translate-x-full";
  }
  return "-translate-x-1/2";
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
    <div className="flex w-full flex-col gap-3">
      <Slider
        aria-label="Memory"
        className="w-full"
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
        <div
          aria-hidden
          className="pointer-events-none absolute inset-x-0 top-1/2 z-10 h-0"
        >
          {options.map((mb) => {
            const percent = percentage(mb);
            const selected = mb === valueMb;
            return (
              <span
                key={mb}
                className={
                  selected
                    ? "absolute top-1/2 h-2.5 w-px -translate-x-1/2 -translate-y-1/2 bg-primary-foreground/80"
                    : "absolute top-1/2 h-2.5 w-px -translate-x-1/2 -translate-y-1/2 bg-muted-foreground/50"
                }
                style={tickLeftStyle(percent)}
              />
            );
          })}
        </div>
      </Slider>
      <div className="relative h-8 w-full">
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
              className={
                selected
                  ? `absolute top-0 ${markTranslateClass(optionIndex, options.length)} cursor-pointer rounded-md bg-primary/15 text-[0.8rem] font-medium whitespace-nowrap text-foreground ${isEnd ? "px-2 py-1" : "px-2.5 py-1"}`
                  : `absolute top-0 ${markTranslateClass(optionIndex, options.length)} cursor-pointer rounded-md text-[0.8rem] whitespace-nowrap text-muted-foreground hover:bg-muted hover:text-foreground ${isEnd ? "px-2 py-1" : "px-2.5 py-1"}`
              }
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
