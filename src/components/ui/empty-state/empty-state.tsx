import type { ReactNode } from "react";

import { emptyStateStyles } from "./empty-state.styles";

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
    <div role="status" aria-label={title} className={emptyStateStyles.root}>
      <div aria-hidden className={emptyStateStyles.icon}>
        {icon}
      </div>
      <div className={emptyStateStyles.body}>
        <h3 className={emptyStateStyles.title}>{title}</h3>
        {description && (
          <p className={emptyStateStyles.description}>{description}</p>
        )}
      </div>
    </div>
  );
}
