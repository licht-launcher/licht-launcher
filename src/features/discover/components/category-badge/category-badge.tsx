import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { CATEGORY_BY_VALUE } from "@/features/discover/lib/filter-options";
import type { ModpackCategory } from "@/lib/generated/ModpackCategory";

import { categoryBadgeStyles } from "./category-badge.styles";

/**
 * A category's icon and name, with a plain border. `children` go after the
 * name (such as a remove icon).
 */
export function CategoryBadge({
  category,
  children,
}: {
  category: ModpackCategory;
  children?: ReactNode;
}) {
  const option = CATEGORY_BY_VALUE.get(category);
  if (!option) return null;
  return (
    <Badge data-category={category} className={categoryBadgeStyles.root}>
      <option.icon aria-hidden />
      {option.label}
      {children}
    </Badge>
  );
}
