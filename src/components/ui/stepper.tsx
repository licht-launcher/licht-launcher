import type { LucideIcon } from "lucide-react";
import { CheckIcon } from "lucide-react";

import { cn } from "@/lib/utils";

export type StepperItem = {
  label: string;
  icon?: LucideIcon;
};

export function Stepper({
  steps,
  currentIndex,
}: {
  steps: StepperItem[];
  currentIndex: number;
}) {
  return (
    <ol
      className="flex flex-wrap items-center gap-2"
      aria-label="Wizard progress"
    >
      {steps.map((step, index) => {
        const active = index === currentIndex;
        const done = index < currentIndex;
        const Icon = step.icon;

        return (
          <li key={step.label} className="flex items-center gap-2">
            {index > 0 ? (
              <span
                aria-hidden
                className={cn(
                  "hidden h-px w-4 sm:block",
                  done || active ? "bg-foreground/30" : "bg-border",
                )}
              />
            ) : null}
            <span
              className={cn(
                "inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-xs font-medium",
                active && "border-border bg-muted text-foreground",
                done && "border-transparent bg-black/20 text-muted-foreground",
                !active &&
                  !done &&
                  "border-transparent bg-black/10 text-muted-foreground",
              )}
              aria-current={active ? "step" : undefined}
            >
              <span
                className={cn(
                  "flex size-4 shrink-0 items-center justify-center rounded-full",
                  active || done
                    ? "bg-success/25 text-success"
                    : "bg-muted/60 text-muted-foreground",
                )}
              >
                {done ? (
                  <CheckIcon aria-hidden className="size-2.5" strokeWidth={3} />
                ) : Icon !== undefined ? (
                  <Icon aria-hidden className="size-2.5" strokeWidth={2.5} />
                ) : (
                  <span className="text-[0.6rem] leading-none">
                    {index + 1}
                  </span>
                )}
              </span>
              {step.label}
            </span>
          </li>
        );
      })}
    </ol>
  );
}
