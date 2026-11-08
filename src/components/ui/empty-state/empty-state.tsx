import type { ReactNode } from "react";

export function EmptyState({
  icon,
  title,
  description,
}: {
  icon: ReactNode;
  title: string;
  description?: string;
}) {
  return (
    <div
      role="status"
      aria-label={title}
      className="flex flex-1 flex-col items-center justify-center gap-3 px-4 py-8 text-center"
    >
      <div
        aria-hidden
        className="rounded-2xl border border-border bg-muted/50 p-3 text-muted-foreground [&_svg]:size-6"
      >
        {icon}
      </div>
      <div className="flex max-w-sm flex-col gap-1">
        <h3 className="text-base font-medium">{title}</h3>
        {description && (
          <p className="text-sm leading-relaxed text-muted-foreground">
            {description}
          </p>
        )}
      </div>
    </div>
  );
}
