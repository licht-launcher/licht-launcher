import { cn } from "cn";
import { Slider as SliderPrimitive } from "radix-ui";
import type * as React from "react";

import { sliderStyles } from "./slider.styles";

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
      className={cn(sliderStyles.root, className)}
      {...props}
    >
      <SliderPrimitive.Track
        data-slot="slider-track"
        className={sliderStyles.track}
      >
        <SliderPrimitive.Range
          data-slot="slider-range"
          className={sliderStyles.range}
        />
      </SliderPrimitive.Track>
      {children}
      <SliderPrimitive.Thumb
        data-slot="slider-thumb"
        aria-label={props["aria-label"]}
        aria-labelledby={props["aria-labelledby"]}
        aria-describedby={props["aria-describedby"]}
        aria-valuetext={props["aria-valuetext"]}
        className={sliderStyles.thumb}
      />
    </SliderPrimitive.Root>
  );
}

export { Slider };
