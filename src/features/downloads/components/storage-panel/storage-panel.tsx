import { useQuery } from "@tanstack/react-query";
import { getStorageSummary } from "@/api/commands";
import { Button } from "@/components/ui/button";
import { type SegmentColor, SegmentedBar } from "@/components/ui/segmented-bar";
import { useStorageLoadingPreview } from "@/features/downloads/hooks/use-storage-loading-preview";
import type { StorageCategory } from "@/lib/generated/StorageCategory";
import { cn } from "@/lib/utils";

import { storagePanelStyles } from "./storage-panel.styles";

const categories: Record<
  StorageCategory,
  { label: string; color: SegmentColor }
> = {
  instances: { label: "Instances", color: "primary" },
  gameFiles: { label: "Game files", color: "warning" },
  natives: { label: "Natives", color: "chart1" },
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

export function StoragePanel({
  active,
  embedded = false,
}: {
  active: boolean;
  embedded?: boolean;
}) {
  const Heading = embedded ? "h4" : "h2";
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
      aria-label="Launcher Storage"
      className={cn(
        storagePanelStyles.root,
        !embedded && storagePanelStyles.card,
      )}
    >
      <div className={storagePanelStyles.header}>
        <Heading className={storagePanelStyles.title}>Launcher Storage</Heading>
        {storage && (
          <span className={storagePanelStyles.total}>
            {formatSize(storage.managedBytes)}
          </span>
        )}
      </div>
      {query.isError && !previewLoading && (
        <div role="alert" className={storagePanelStyles.legend}>
          <span>Could not load storage information.</span>
          <Button
            variant={embedded ? "ghost" : "outline"}
            size={embedded ? "default" : "sm"}
            onClick={() => void query.refetch()}
          >
            Retry
          </Button>
        </div>
      )}
      {(storage || loading) && (
        <SegmentedBar
          label="Launcher storage"
          compact={embedded}
          loading={loading}
          segments={segments}
        />
      )}
    </section>
  );
}
