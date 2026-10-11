import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export function ChoiceCard({
  title,
  description,
  icon,
  iconNeutral = false,
  selected,
  onSelect,
  className,
  children,
}: {
  title: string;
  description?: string;
  icon?: ReactNode;
  /** Neutral well for brand/loader images; brighter instance purple for Lucide icons. */
  iconNeutral?: boolean;
  selected: boolean;
  onSelect: () => void;
  className?: string;
  children?: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      className={cn(
        "flex h-auto w-full cursor-pointer flex-col items-start gap-3 rounded-lg border px-4 py-4 text-left outline-none select-none",
        "transition-[background-color,border-color,box-shadow] duration-200 ease-out",
        "focus-visible:ring-3 focus-visible:ring-ring/50",
        selected
          ? "border-primary/45 bg-primary/15 text-foreground"
          : "border-border/60 bg-black/20 text-foreground hover:bg-muted/50",
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
                : "bg-instance/20 text-instance",
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
