import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { CATEGORY_BY_VALUE } from "@/features/discover/lib/filter-options";
import type { ModpackCategory } from "@/lib/generated/ModpackCategory";
import { cn } from "@/lib/utils";

import { categoryBadgeStyles } from "./category-badge.styles";

/**
 * A category's icon and name, in that category's own color. `children` go
 * after the name (such as a remove icon).
 */
export function CategoryBadge({
  category,
  className,
  children,
}: {
  category: ModpackCategory;
  className?: string;
  children?: ReactNode;
}) {
  const option = CATEGORY_BY_VALUE.get(category);
  if (!option) return null;
  return (
    <Badge
      data-category={category}
      className={cn(categoryBadgeStyles.tones[category], className)}
    >
      <option.icon aria-hidden />
      {option.label}
      {children}
    </Badge>
  );
}
