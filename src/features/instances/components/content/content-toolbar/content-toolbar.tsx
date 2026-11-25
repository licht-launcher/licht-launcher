import { RefreshCwIcon, SearchIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ContentFilterSelect } from "@/features/instances/components/content/content-filter-select";
import type { ContentFilter } from "@/features/instances/lib/content-table";

import { contentToolbarStyles as styles } from "./content-toolbar.styles";

/** Search, kind filter and refresh for an instance's content list. */
export function ContentToolbar({
  search,
  onSearchChange,
  filter,
  onFilterChange,
  refreshing,
  onRefresh,
}: {
  search: string;
  onSearchChange: (search: string) => void;
  filter: ContentFilter;
  onFilterChange: (filter: ContentFilter) => void;
  refreshing: boolean;
  onRefresh: () => void;
}) {
  return (
    <>
      <Input
        aria-label="Search content"
        className={styles.searchInput}
        placeholder="Search content"
        value={search}
        startIcon={<SearchIcon aria-hidden />}
        onChange={(event) => onSearchChange(event.target.value)}
      />
      <ContentFilterSelect value={filter} onValueChange={onFilterChange} />
      <Button
        type="button"
        variant="outline"
        size="icon"
        aria-label="Refresh content"
        disabled={refreshing}
        onClick={onRefresh}
      >
        <RefreshCwIcon
          aria-hidden
          className={refreshing ? styles.refreshing : undefined}
        />
      </Button>
    </>
  );
}
