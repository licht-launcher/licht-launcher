import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

const iconTones = {
  instance: "bg-instance/20 text-instance",
  warning: "bg-warning/20 text-warning",
  cool: "bg-accent-cool/20 text-accent-cool",
  file: "bg-accent-file/20 text-accent-file",
} as const;

export type ChoiceCardIconTone = keyof typeof iconTones;

export function ChoiceCard({
  title,
  description,
  icon,
  iconNeutral = false,
  iconTone = "instance",
  size = "default",
  selected,
  disabled = false,
  onSelect,
  className,
  children,
}: {
  title: string;
  description?: string;
  icon?: ReactNode;
  /** Neutral well for brand/loader images. */
  iconNeutral?: boolean;
  /** Accent well for Lucide icons when not neutral. */
  iconTone?: ChoiceCardIconTone;
  /** `lg` gives a roomier card for primary, one-of-few choices. */
  size?: "default" | "lg";
  selected: boolean;
  disabled?: boolean;
  onSelect: () => void;
  className?: string;
  children?: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      disabled={disabled}
      className={cn(
        "flex h-auto w-full cursor-pointer flex-col items-start gap-3 rounded-lg border px-3 py-3 text-left text-foreground outline-none select-none",
        "transition-[background-color,border-color,box-shadow] duration-200 ease-out",
        "focus-visible:ring-3 focus-visible:ring-ring/50",
        "disabled:pointer-events-none disabled:opacity-50",
        "border-border/60 ring-3",
        size === "lg" && "min-h-40 gap-4 px-4 py-4",
        selected ? "bg-muted ring-primary" : "ring-primary/0",
        className,
      )}
      onClick={onSelect}
    >
      <span className="flex flex-col items-start w-full gap-2">
        <div className="flex items-center gap-2">
          {icon !== undefined ? (
            <span
              className={cn(
                "flex shrink-0 items-center justify-center overflow-hidden rounded-md transition-colors duration-200 ease-out",
                size === "lg" ? "size-10" : "size-7",
                iconNeutral
                  ? "bg-black/20 text-muted-foreground"
                  : iconTones[iconTone],
              )}
            >
              {icon}
            </span>
          ) : null}
          <span className={cn("block text-sm", size === "lg" && "text-base")}>
            {title}
          </span>
        </div>

        {description !== undefined ? (
          <span className="mt-0.5 block text-xs font-normal text-muted-foreground">
            {description}
          </span>
        ) : null}
        {children}
      </span>
    </button>
  );
}
