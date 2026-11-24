import { BlocksIcon, ImageIcon, SparklesIcon } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ContentFilter } from "@/features/instances/lib/content-table";

import { contentFilterSelectStyles } from "./content-filter-select.styles";

const FILTERS: readonly ContentFilter[] = [
  "all",
  "mods",
  "resourcePacks",
  "shaderPacks",
];

function isContentFilter(value: string): value is ContentFilter {
  return FILTERS.some((filter) => filter === value);
}

/** Picks which kind of content a content list shows. */
export function ContentFilterSelect({
  value,
  onValueChange,
}: {
  value: ContentFilter;
  onValueChange: (value: ContentFilter) => void;
}) {
  return (
    <Select
      value={value}
      onValueChange={(next) => {
        if (isContentFilter(next)) onValueChange(next);
      }}
    >
      <SelectTrigger
        aria-label="Filter content"
        className={contentFilterSelectStyles.trigger}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">
          <BlocksIcon aria-hidden />
          All content
        </SelectItem>
        <SelectItem value="mods">
          <BlocksIcon aria-hidden />
          Mods
        </SelectItem>
        <SelectItem value="resourcePacks">
          <ImageIcon aria-hidden />
          Resource Packs
        </SelectItem>
        <SelectItem value="shaderPacks">
          <SparklesIcon aria-hidden />
          Shader Packs
        </SelectItem>
      </SelectContent>
    </Select>
  );
}
