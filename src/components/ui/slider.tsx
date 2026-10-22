import { cn } from "cn";
import { Slider as SliderPrimitive } from "radix-ui";
import type * as React from "react";

function Slider({
  className,
  defaultValue,
  value,
  children,
  ...props
}: React.ComponentProps<typeof SliderPrimitive.Root>) {
  return (
    <SliderPrimitive.Root
      data-slot="slider"
      defaultValue={defaultValue}
      value={value}
      className={cn(
        "relative flex w-full touch-none items-center select-none data-disabled:opacity-50",
        className,
      )}
      {...props}
    >
      <SliderPrimitive.Track
        data-slot="slider-track"
        className="relative h-1.5 w-full grow overflow-hidden rounded-full bg-muted"
      >
        <SliderPrimitive.Range
          data-slot="slider-range"
          className="absolute h-full bg-primary"
        />
      </SliderPrimitive.Track>
      {children}
      <SliderPrimitive.Thumb
        data-slot="slider-thumb"
        aria-labelledby={props["aria-labelledby"]}
        aria-describedby={props["aria-describedby"]}
        aria-valuetext={props["aria-valuetext"]}
        className="relative z-20 block size-4 rounded-full border border-primary bg-white outline-none focus-visible:ring-3 focus-visible:ring-white/80"
      />
    </SliderPrimitive.Root>
  );
}

export { Slider };
