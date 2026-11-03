import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/** A titled block of the create form: icon, title, optional description, then content. */
export function FormSection({
  title,
  description,
  divided = true,
  children,
}: {
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
