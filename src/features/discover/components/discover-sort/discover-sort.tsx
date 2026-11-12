import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SORT_OPTIONS } from "@/features/discover/lib/filter-options";
import type { ModpackSort } from "@/lib/generated/ModpackSort";

import { discoverSortStyles } from "./discover-sort.styles";

/** Order of the store list, shown next to the search box. */
export function DiscoverSort({
  value,
  onChange,
}: {
  value: ModpackSort;
  onChange: (sort: ModpackSort) => void;
}) {
  return (
    <Select
      value={value}
      onValueChange={(next) => {
        const sort = SORT_OPTIONS.find((option) => option.value === next);
        if (sort) onChange(sort.value);
      }}
    >
      <SelectTrigger
        size="sm"
        aria-label="Sort by"
        className={discoverSortStyles.trigger}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {SORT_OPTIONS.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            <option.icon aria-hidden className={discoverSortStyles.icon} />
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
