"use client";

import { cn } from "cn";
import { Progress as ProgressPrimitive } from "radix-ui";
import type * as React from "react";

import { progressStyles } from "./progress.styles";

function Progress({
  className,
  value,
  ...props
}: React.ComponentProps<typeof ProgressPrimitive.Root>) {
  const isIndeterminate = value == null;

  return (
    <ProgressPrimitive.Root
      data-slot="progress"
      className={cn(progressStyles.root, className)}
      value={value}
      {...props}
    >
      <ProgressPrimitive.Indicator
        data-slot="progress-indicator"
        className={cn(
          progressStyles.indicator,
          isIndeterminate
            ? progressStyles.indicatorIndeterminate
            : progressStyles.indicatorDeterminate,
        )}
        style={
          isIndeterminate
            ? undefined
            : { transform: `translateX(-${100 - (value ?? 0)}%)` }
        }
      />
    </ProgressPrimitive.Root>
  );
}

export { Progress };
