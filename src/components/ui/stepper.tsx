import { CheckIcon } from "lucide-react";

import { cn } from "@/lib/utils";

export type StepperItem = {
  label: string;
};

export function Stepper({
  steps,
  currentIndex,
  onSelect,
}: {
  steps: StepperItem[];
  currentIndex: number;
  /** Called with the index of a finished step the user wants to go back to. */
  onSelect?: (index: number) => void;
}) {
  return (
    <ol className="flex w-full items-start" aria-label="Wizard progress">
      {steps.map((step, index) => {
        const active = index === currentIndex;
        const done = index < currentIndex;
        const last = index === steps.length - 1;

        const marker = (
          <span
            data-testid={done ? "step-done" : undefined}
            className={cn(
              "flex size-8 shrink-0 items-center justify-center rounded-full border text-sm font-medium transition-colors",
              done && "border-transparent bg-instance/20 text-instance",
              active && "border-instance bg-instance/20 text-foreground",
              !(done || active) &&
                "border-border bg-black/10 text-muted-foreground",
            )}
          >
            {done ? (
              <CheckIcon aria-hidden className="size-4" strokeWidth={3} />
            ) : (
              index + 1
            )}
          </span>
        );
        const label = (
          <span
            className={cn(
              "text-xs font-medium",
              active ? "text-foreground" : "text-muted-foreground",
            )}
          >
            {step.label}
          </span>
        );

        return (
          <li
            key={step.label}
            className={cn("flex items-start", last ? "flex-none" : "flex-1")}
          >
            {done && onSelect !== undefined ? (
              <button
                type="button"
                className="flex shrink-0 cursor-pointer flex-col items-center gap-1.5 rounded-md outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                onClick={() => {
                  onSelect(index);
                }}
              >
                {marker}
                {label}
              </button>
            ) : (
              <div
                className="flex shrink-0 flex-col items-center gap-1.5"
                aria-current={active ? "step" : undefined}
              >
                {marker}
                {label}
              </div>
            )}
            {last ? null : (
              <span
                aria-hidden
                className={cn(
                  "mx-3 mt-4 h-px min-w-6 flex-1 transition-colors",
                  done ? "bg-instance/60" : "bg-border",
                )}
              />
            )}
          </li>
        );
      })}
    </ol>
  );
}
