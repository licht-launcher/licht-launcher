import { ChevronRightIcon } from "lucide-react";
import type { ReactNode } from "react";

import { type Tone, tones } from "@/features/new-instance/components/highlight";
import { cn } from "@/lib/utils";

export function SourceRow({
  tone,
  icon,
  title,
  description,
  disabled,
  onSelect,
}: {
  tone: Tone;
  icon: ReactNode;
  title: string;
  description: ReactNode;
  disabled: boolean;
  onSelect: () => void;
}) {
  return (
    <li>
      <button
        type="button"
        disabled={disabled}
        onClick={onSelect}
        className={cn(
          "group flex w-full cursor-pointer items-center gap-4 rounded-xl border border-border/60 px-3 py-3 text-left outline-none",
          "transition-[background-color,border-color,transform] duration-200 ease-out",
          "hover:border-border hover:bg-muted/50 focus-visible:ring-3 focus-visible:ring-ring/50",
          "disabled:pointer-events-none disabled:opacity-60",
        )}
      >
        <span
          className={cn(
            "flex size-11 shrink-0 items-center justify-center rounded-xl transition-transform duration-200 ease-out motion-safe:group-hover:scale-105",
            tones[tone].well,
          )}
        >
          {icon}
        </span>
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="text-[15px] font-medium tracking-tight">
            {title}
          </span>
          <span className="text-xs leading-snug text-muted-foreground">
            {description}
          </span>
        </span>

        <ChevronRightIcon
          aria-hidden
          className="size-5 shrink-0 text-muted-foreground transition-[transform,color] duration-200 ease-out group-hover:text-foreground motion-safe:group-hover:translate-x-0.5"
        />
      </button>
    </li>
  );
}
