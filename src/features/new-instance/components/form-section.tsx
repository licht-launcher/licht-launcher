import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/** A titled block of the create form: icon, title, optional description, then content. */
export function FormSection({
  icon: Icon,
  title,
  description,
  divided = true,
  children,
}: {
  icon: LucideIcon;
  title: string;
  description?: string;
  /** Draws a divider above the section; the first one omits it. */
  divided?: boolean;
  children: ReactNode;
}) {
  return (
    <section
      className={cn(
        "flex flex-col gap-3",
        divided && "border-t border-border/40 pt-5",
      )}
    >
      <div className="flex items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
          <Icon aria-hidden className="size-4" />
        </div>
        <div>
          <h3 className="text-sm font-medium">{title}</h3>
          {description === undefined ? null : (
            <p className="text-xs text-muted-foreground">{description}</p>
          )}
        </div>
      </div>
      {children}
    </section>
  );
}
