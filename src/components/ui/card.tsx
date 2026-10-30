import { type ComponentProps, useId } from "react";

import { cn } from "@/lib/utils";

export function Card({
  title,
  className,
  ...props
}: ComponentProps<"section">) {
  const titleId = useId();
  const content = (
    <section
      data-slot="card"
      aria-labelledby={title && !props["aria-label"] ? titleId : undefined}
      className={cn(
        "flex min-w-0 flex-col gap-4 rounded-xl border border-border bg-muted/60 p-4 leading-snug text-card-foreground [&_p]:leading-snug",
        className,
      )}
      {...props}
    />
  );

  return title ? (
    <div className="flex min-w-0 flex-col gap-2">
      <h3
        id={titleId}
        className="px-3 text-xs font-normal text-muted-foreground"
      >
        {title}
      </h3>
      {content}
    </div>
  ) : (
    content
  );
}
