import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

import { marqueeTextStyles } from "./marquee-text.styles";

/**
 * One line of text that scrolls back and forth when it is wider than its box.
 * Text that fits stays still, and so does all text under reduced motion.
 */
export function MarqueeText({
  className,
  children,
  ...props
}: ComponentProps<"span">) {
  return (
    <span
      data-slot="marquee-text"
      className={cn(marqueeTextStyles.root, className)}
      {...props}
    >
      <span className={marqueeTextStyles.text}>{children}</span>
    </span>
  );
}
