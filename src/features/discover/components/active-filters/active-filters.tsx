import { XIcon } from "lucide-react";
import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { CategoryBadge } from "@/features/discover/components/category-badge";
import { LoaderBadge } from "@/features/discover/components/loader-badge";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import {
  CATEGORY_BY_VALUE,
  LOADER_LABELS,
} from "@/features/discover/lib/filter-options";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";

import { activeFiltersStyles } from "./active-filters.styles";

type ActiveFilter = {
  key: string;
  label: string;
  badge: ReactNode;
  clear: Partial<ModpackSearchFilters>;
};

function activeFilters(filters: ModpackSearchFilters): ActiveFilter[] {
  const remove = (
    <XIcon aria-hidden className={activeFiltersStyles.removeIcon} />
  );
  const active: ActiveFilter[] = [];
  if (filters.source !== null) {
    const label = SOURCE_LABELS[filters.source];
    active.push({
      key: "source",
      label,
      badge: (
        <Badge>
          {label}
          {remove}
        </Badge>
      ),
      clear: { source: null },
    });
  }
  if (filters.gameVersion !== null) {
    const label = filters.gameVersion;
    active.push({
      key: "gameVersion",
      label,
      badge: (
        <Badge className={activeFiltersStyles.version}>
          {label}
          {remove}
        </Badge>
      ),
      clear: { gameVersion: null },
    });
  }
  for (const loader of filters.loaders) {
    active.push({
      key: `loader-${loader}`,
      label: LOADER_LABELS.get(loader) ?? loader,
      badge: <LoaderBadge loader={loader}>{remove}</LoaderBadge>,
      clear: {
        loaders: filters.loaders.filter((other) => other !== loader),
      },
    });
  }
  for (const category of filters.categories) {
    active.push({
      key: `category-${category}`,
      label: CATEGORY_BY_VALUE.get(category)?.label ?? category,
      badge: <CategoryBadge category={category}>{remove}</CategoryBadge>,
      clear: {
        categories: filters.categories.filter((other) => other !== category),
      },
    });
  }
  return active;
}

/** The filters narrowing the search, each one removable with a click. */
export function ActiveFilters({
  filters,
  onChange,
}: {
  filters: ModpackSearchFilters;
  onChange: (filters: ModpackSearchFilters) => void;
}) {
  const active = activeFilters(filters);
  if (active.length === 0) return null;

  return (
    <ul aria-label="Active filters" className={activeFiltersStyles.root}>
      {active.map((filter) => (
        <li key={filter.key} className={activeFiltersStyles.item}>
          <button
            type="button"
            aria-label={`Remove ${filter.label} filter`}
            className={activeFiltersStyles.remove}
            onClick={() => onChange({ ...filters, ...filter.clear })}
          >
            {filter.badge}
          </button>
        </li>
      ))}
    </ul>
  );
}
