import { type ReactNode, useEffect, useState } from "react";

import { cn } from "@/lib/utils";

/** A little over the 200ms transition, so the content is never cut short. */
const COLLAPSE_MS = 250;

/**
 * Grows its content open and shrinks it shut instead of popping in and out.
 * The content stays mounted until it has finished shrinking, and is `inert`
 * (not focusable or clickable) while it is closing.
 */
export function Reveal({
  open,
  className,
  children,
}: {
  open: boolean;
  className?: string;
  children: ReactNode;
}) {
  const [mounted, setMounted] = useState(open);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    if (open) {
      setMounted(true);
      // The collapsed state has to be painted once before growing, or the
      // browser has nothing to animate from. One frame is not enough for that.
      let inner = 0;
      const outer = requestAnimationFrame(() => {
        inner = requestAnimationFrame(() => {
          setExpanded(true);
        });
      });
      return () => {
        cancelAnimationFrame(outer);
        cancelAnimationFrame(inner);
      };
    }
    setExpanded(false);
    const timer = setTimeout(() => {
      setMounted(false);
    }, COLLAPSE_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [open]);

  if (!mounted) {
    return null;
  }

  return (
    <div
      data-slot="reveal"
      data-state={expanded ? "open" : "closed"}
      inert={!expanded}
      className={cn(
        "grid transition-[grid-template-rows,opacity] duration-200 ease-out motion-reduce:transition-none",
        expanded ? "grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0",
        className,
      )}
    >
      {/* The side room keeps focus outlines from being clipped by the overflow. */}
      <div
        data-slot="reveal-clip"
        className="-mx-1 min-h-0 overflow-hidden px-1"
      >
        <div className="py-1">{children}</div>
      </div>
    </div>
  );
}
