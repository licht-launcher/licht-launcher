import { useQuery } from "@tanstack/react-query";
import { XIcon } from "lucide-react";
import { type ReactNode, useId } from "react";

import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  MODPACK_SOURCES,
  parseModpackSource,
  SOURCE_LABELS,
} from "@/features/discover/lib/discover-search";
import {
  CATEGORY_OPTIONS,
  LOADER_OPTIONS,
  SORT_OPTIONS,
} from "@/features/discover/lib/filter-options";
import { versionsQueryOptions } from "@/features/instances/lib/versions-query";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";

import { discoverFiltersStyles } from "./discover-filters.styles";

/** Select value for "no narrowing"; Radix Select does not allow an empty value. */
const ANY = "any";

function optionValue<T extends string>(
  options: readonly { value: T }[],
  value: string,
): T | null {
  return options.find((option) => option.value === value)?.value ?? null;
}

/** Discover's left sidebar: sort plus filters that narrow the search. */
export function DiscoverFilters({
  filters,
  onChange,
}: {
  filters: ModpackSearchFilters;
  onChange: (filters: ModpackSearchFilters) => void;
}) {
  const versions = useQuery(versionsQueryOptions);
  const versionChoices = [
    { value: "", label: "Any version" },
    ...createVersionChoices(versions.data ?? [], false).map((version) => ({
      value: version.id,
      label: version.id,
    })),
  ];
  const narrowed =
    filters.gameVersion !== null ||
    filters.loader !== null ||
    filters.category !== null ||
    filters.source !== null;

  function update(patch: Partial<ModpackSearchFilters>) {
    onChange({ ...filters, ...patch });
  }

  return (
    <aside aria-label="Filters" className={discoverFiltersStyles.root}>
      <ScrollArea className={discoverFiltersStyles.scrollArea}>
        <div className={discoverFiltersStyles.content}>
          <FilterGroup title="Sort by">
            {(labelId) => (
              <Select
                value={filters.sort}
                onValueChange={(value) =>
                  update({
                    sort: optionValue(SORT_OPTIONS, value) ?? "downloads",
                  })
                }
              >
                <SelectTrigger
                  aria-labelledby={labelId}
                  className={discoverFiltersStyles.trigger}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {SORT_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </FilterGroup>
          <FilterGroup title="Source">
            {(labelId) => (
              <Select
                value={filters.source ?? ANY}
                onValueChange={(value) =>
                  update({ source: parseModpackSource(value) })
                }
              >
                <SelectTrigger
                  aria-labelledby={labelId}
                  className={discoverFiltersStyles.trigger}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={ANY}>Modrinth and CurseForge</SelectItem>
                  {MODPACK_SOURCES.map((source) => (
                    <SelectItem key={source} value={source}>
                      {SOURCE_LABELS[source]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </FilterGroup>
          <FilterGroup title="Game version">
            {() => (
              <Combobox
                aria-label="Game version"
                options={versionChoices}
                value={filters.gameVersion ?? ""}
                disabled={versions.isPending || versions.isError}
                placeholder={
                  versions.isPending
                    ? "Loading versions…"
                    : versions.isError
                      ? "Versions unavailable"
                      : "Any version"
                }
                emptyText="No versions match."
                onValueChange={(value) =>
                  update({ gameVersion: value === "" ? null : value })
                }
              />
            )}
          </FilterGroup>
          <FilterGroup title="Loader">
            {(labelId) => (
              <Select
                value={filters.loader ?? ANY}
                onValueChange={(value) =>
                  update({ loader: optionValue(LOADER_OPTIONS, value) })
                }
              >
                <SelectTrigger
                  aria-labelledby={labelId}
                  className={discoverFiltersStyles.trigger}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={ANY}>Any loader</SelectItem>
                  {LOADER_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </FilterGroup>
          <FilterGroup title="Category">
            {(labelId) => (
              <ul
                aria-labelledby={labelId}
                className={discoverFiltersStyles.categories}
              >
                {CATEGORY_OPTIONS.map((option) => {
                  const pressed = filters.category === option.value;
                  return (
                    <li key={option.value}>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        aria-pressed={pressed}
                        className={discoverFiltersStyles.category}
                        onClick={() =>
                          update({ category: pressed ? null : option.value })
                        }
                      >
                        {option.label}
                      </Button>
                    </li>
                  );
                })}
              </ul>
            )}
          </FilterGroup>
          {narrowed && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className={discoverFiltersStyles.clear}
              onClick={() =>
                update({
                  gameVersion: null,
                  loader: null,
                  category: null,
                  source: null,
                })
              }
            >
              <XIcon aria-hidden />
              Clear filters
            </Button>
          )}
        </div>
      </ScrollArea>
    </aside>
  );
}

function FilterGroup({
  title,
  children,
}: {
  title: string;
  children: (labelId: string) => ReactNode;
}) {
  const labelId = useId();
  return (
    <section aria-labelledby={labelId} className={discoverFiltersStyles.group}>
      <h2 id={labelId} className={discoverFiltersStyles.heading}>
        {title}
      </h2>
      {children(labelId)}
    </section>
  );
}
