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
        "flex h-auto w-full cursor-pointer flex-col items-start gap-3 rounded-lg border px-4 py-4 text-left outline-none select-none",
        "transition-[background-color,border-color,box-shadow] duration-200 ease-out",
        "focus-visible:ring-3 focus-visible:ring-ring/50",
        "disabled:pointer-events-none disabled:opacity-50",
        "border-border/60 ring-2",
        selected
          ? "bg-muted text-foreground shadow-sm ring-primary"
          : "bg-black/20 text-foreground ring-primary/0 hover:bg-muted/50",
        className,
      )}
      onClick={onSelect}
    >
      <span className="flex w-full items-start gap-3">
        {icon !== undefined ? (
          <span
            className={cn(
              "flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-md transition-colors duration-200 ease-out [&_img]:size-5 [&_img]:object-contain [&_svg]:size-5",
              iconNeutral
                ? "bg-black/20 text-muted-foreground"
                : iconTones[iconTone],
            )}
          >
            {icon}
          </span>
        ) : null}
        <span className="min-w-0 flex-1">
          <span className="block text-sm font-medium">{title}</span>
          {description !== undefined ? (
            <span className="mt-0.5 block text-xs font-normal text-muted-foreground">
              {description}
            </span>
          ) : null}
          {children}
        </span>
      </span>
    </button>
  );
}
