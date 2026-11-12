import { useQuery } from "@tanstack/react-query";
import { ChevronDownIcon } from "lucide-react";
import { type ReactNode, useId } from "react";

import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Combobox } from "@/components/ui/combobox";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { CATEGORY_TONES } from "@/features/discover/components/category-badge/category-badge.styles";
import {
  parseModpackSource,
  SOURCE_LABELS,
  toggleCategory,
} from "@/features/discover/lib/discover-search";
import {
  CATEGORY_OPTIONS,
  LOADER_OPTIONS,
} from "@/features/discover/lib/filter-options";
import { versionsQueryOptions } from "@/features/instances/lib/versions-query";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import { PlatformIcon } from "@/features/new-instance/assets/platform-icons";
import type { ModpackSearchFilters } from "@/lib/generated/ModpackSearchFilters";
import type { ModpackSource } from "@/lib/generated/ModpackSource";
import { cn } from "@/lib/utils";

import { discoverFiltersStyles } from "./discover-filters.styles";

/** Select value for "no narrowing"; Radix Select does not allow an empty value. */
const ANY = "any";

function optionValue<T extends string>(
  options: readonly { value: T }[],
  value: string,
): T | null {
  return options.find((option) => option.value === value)?.value ?? null;
}

/** Discover's left sidebar: filters that narrow the search. */
export function DiscoverFilters({
  filters,
  sources,
  onChange,
}: {
  filters: ModpackSearchFilters;
  /** Sources this build can search; the Source group needs at least two. */
  sources: readonly ModpackSource[];
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
  function update(patch: Partial<ModpackSearchFilters>) {
    onChange({ ...filters, ...patch });
  }

  return (
    <aside aria-label="Filters" className={discoverFiltersStyles.root}>
      <ScrollArea className={discoverFiltersStyles.scrollArea}>
        <div className={discoverFiltersStyles.content}>
          {sources.length > 1 && (
            <FilterGroup title="Source">
              {(labelId) => (
                <Select
                  value={filters.source ?? ANY}
                  onValueChange={(value) =>
                    update({ source: parseModpackSource(value) })
                  }
                >
                  <SelectTrigger
                    size="sm"
                    aria-labelledby={labelId}
                    className={discoverFiltersStyles.trigger}
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={ANY}>Any source</SelectItem>
                    {sources.map((source) => (
                      <SelectItem key={source} value={source}>
                        {SOURCE_LABELS[source]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </FilterGroup>
          )}
          <FilterGroup title="Game version">
            {() => (
              <Combobox
                aria-label="Game version"
                size="sm"
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
                  size="sm"
                  aria-labelledby={labelId}
                  className={discoverFiltersStyles.trigger}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={ANY}>Any loader</SelectItem>
                  {LOADER_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      <PlatformIcon
                        platform={option.value}
                        className={discoverFiltersStyles.loaderIcon}
                      />
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
                  const pressed = filters.categories.includes(option.value);
                  return (
                    <li key={option.value}>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        aria-pressed={pressed}
                        className={cn(
                          discoverFiltersStyles.category,
                          pressed && CATEGORY_TONES[option.value],
                        )}
                        onClick={() =>
                          update({
                            categories: toggleCategory(
                              filters.categories,
                              option.value,
                            ),
                          })
                        }
                      >
                        <option.icon aria-hidden />
                        {option.label}
                      </Button>
                    </li>
                  );
                })}
              </ul>
            )}
          </FilterGroup>
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
    <Collapsible defaultOpen asChild>
      <section
        aria-labelledby={labelId}
        className={discoverFiltersStyles.group}
      >
        <h2 id={labelId}>
          <CollapsibleTrigger className={discoverFiltersStyles.heading}>
            <ChevronDownIcon
              aria-hidden
              className={discoverFiltersStyles.chevron}
            />
            {title}
          </CollapsibleTrigger>
        </h2>
        <CollapsibleContent className={discoverFiltersStyles.groupContent}>
          {children(labelId)}
        </CollapsibleContent>
      </section>
    </Collapsible>
  );
}
