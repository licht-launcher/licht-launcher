import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import type { VersionSummary } from "@/lib/generated/VersionSummary";
import { cn } from "@/lib/utils";

const ROW_HEIGHT = 36;

export function VersionList({
  versions,
  selected,
  pending,
  error,
  includeSnapshots,
  onSelect,
  onIncludeSnapshotsChange,
  onRetry,
}: {
  versions: VersionSummary[];
  selected: string;
  pending: boolean;
  error: boolean;
  includeSnapshots: boolean;
  onSelect: (versionId: string) => void;
  onIncludeSnapshotsChange: (include: boolean) => void;
  onRetry: () => void;
}) {
  const [query, setQuery] = useState("");
  const viewportRef = useRef<HTMLDivElement>(null);

  const needle = query.trim().toLowerCase();
  const visible = createVersionChoices(versions, includeSnapshots).filter(
    (version) => needle === "" || version.id.toLowerCase().includes(needle),
  );

  const virtualizer = useVirtualizer({
    count: visible.length,
    getScrollElement: () => viewportRef.current,
    estimateSize: () => ROW_HEIGHT,
    getItemKey: (index) => visible[index]?.id ?? index,
    overscan: 6,
  });

  if (pending) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Spinner className="size-4" />
        Loading versions…
      </div>
    );
  }

  if (error && versions.length === 0) {
    return (
      <div className="flex flex-col gap-2">
        <p className="text-sm text-muted-foreground">
          Could not load the version list. Check your connection and try again.
        </p>
        <Button
          type="button"
          variant="outline"
          className="self-start"
          onClick={onRetry}
        >
          Retry
        </Button>
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-col gap-2">
      <div className="flex items-center gap-3">
        <Input
          aria-label="Search versions"
          placeholder="Search versions"
          autoComplete="off"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
        />
        <div className="flex shrink-0 items-center gap-2 text-sm text-muted-foreground">
          <Switch
            id="include-snapshots"
            aria-label="Include snapshots"
            checked={includeSnapshots}
            onCheckedChange={onIncludeSnapshotsChange}
          />
          <label htmlFor="include-snapshots">Snapshots</label>
        </div>
      </div>
      <div
        ref={viewportRef}
        data-slot="version-viewport"
        className="h-72 overflow-y-auto rounded-lg border border-border/60 bg-black/20"
      >
        {visible.length === 0 ? (
          <p className="p-3 text-sm text-muted-foreground">
            No versions match.
          </p>
        ) : (
          <ul
            aria-label="Game version"
            // Rows are absolutely placed; only the visible window is rendered.
            className="relative w-full"
            style={{ height: virtualizer.getTotalSize() }}
          >
            {virtualizer.getVirtualItems().map((row) => {
              const version = visible[row.index];
              if (version === undefined) {
                return null;
              }
              const isSelected = version.id === selected;
              return (
                <li
                  key={row.key}
                  className="absolute inset-x-0 top-0"
                  style={{
                    height: row.size,
                    transform: `translateY(${row.start}px)`,
                  }}
                >
                  <button
                    type="button"
                    aria-pressed={isSelected}
                    className={cn(
                      "flex size-full cursor-pointer items-center px-3 text-left text-sm outline-none transition-colors",
                      "hover:bg-muted/50 focus-visible:bg-muted/50",
                      isSelected && "bg-muted font-medium",
                    )}
                    onClick={() => {
                      onSelect(version.id);
                    }}
                  >
                    {version.id}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
