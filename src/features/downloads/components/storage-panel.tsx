import { useQuery } from "@tanstack/react-query";
import { HardDrive } from "lucide-react";
import { Button } from "@/components/ui/button";
import { type SegmentColor, SegmentedBar } from "@/components/ui/segmented-bar";
import { useStorageLoadingPreview } from "@/features/downloads/hooks/use-storage-loading-preview";
import { getStorageSummary } from "@/lib/commands";
import type { StorageCategory } from "@/lib/generated/StorageCategory";

const categories: Record<
  StorageCategory,
  { label: string; color: SegmentColor }
> = {
  instances: { label: "Instances", color: "primary" },
  gameFiles: { label: "Game files", color: "warning" },
  java: { label: "Java", color: "violet" },
  cache: { label: "Cache", color: "chart2" },
  other: { label: "Other", color: "chart3" },
};

function formatSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function StoragePanel({ active }: { active: boolean }) {
  const previewLoading = useStorageLoadingPreview();
  const query = useQuery({
    queryKey: ["storage-summary"],
    queryFn: getStorageSummary,
    enabled: !previewLoading,
    retry: false,
    staleTime: 5000,
    // Measuring managed data walks file metadata, so avoid frequent full scans.
    refetchInterval: active ? 30_000 : 60_000,
  });
  const storage = previewLoading ? undefined : query.data;
  const loading = previewLoading || query.isPending;
  const segments = storage
    ? storage.categories.map((usage) => ({
        label: categories[usage.category].label,
        color: categories[usage.category].color,
        value: usage.bytes,
        valueLabel: formatSize(usage.bytes),
      }))
    : Object.values(categories).map((category) => ({
        ...category,
        value: 0,
        valueLabel: "",
      }));

  return (
    <section
      aria-label="Launcher Data"
      className="flex min-w-0 flex-col gap-3 rounded-lg border border-border/60 bg-black/20 p-4"
    >
      <div className="flex items-center justify-between gap-3">
        <h2 className="flex items-center gap-2 text-sm font-medium">
          <HardDrive aria-hidden className="size-4 text-muted-foreground" />
          Launcher Data
        </h2>
        {storage && (
          <span className="text-sm font-medium tabular-nums">
            {formatSize(storage.managedBytes)}
          </span>
        )}
      </div>
      {query.isError && !previewLoading && (
        <div role="alert" className="flex flex-wrap items-center gap-2 text-sm">
          <span>Could not load storage information.</span>
          <Button
            variant="outline"
            size="sm"
            onClick={() => void query.refetch()}
          >
            Retry
          </Button>
        </div>
      )}
      {(storage || loading) && (
        <SegmentedBar
          label="Launcher storage"
          loading={loading}
          segments={segments}
        />
      )}
    </section>
  );
}
