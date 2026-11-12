import { useVirtualizer } from "@tanstack/react-virtual";
import { DownloadIcon, LayersIcon, TriangleAlertIcon } from "lucide-react";
import { useRef, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { LoaderBadge } from "@/features/discover/components/loader-badge";
import { useModpackVersions } from "@/features/discover/hooks/use-modpacks";
import {
  formatDownloads,
  formatTimeAgo,
  formatUpdatedAt,
} from "@/features/discover/lib/format";
import { friendlyError, messageOf } from "@/lib/errors";
import type { ModpackSource } from "@/lib/generated/ModpackSource";
import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";
import type { ModpackVersionType } from "@/lib/generated/ModpackVersionType";
import { cn } from "@/lib/utils";

import { modpackVersionsStyles } from "./modpack-versions.styles";

/** Initial row height guess; real heights are measured once rows render. */
const ROW_ESTIMATE_PX = 52;

const TYPE_LABELS: Record<ModpackVersionType, string> = {
  release: "Release",
  beta: "Beta",
  alpha: "Alpha",
};

/** Every version of a pack, newest first, each with its own Install button. */
export function ModpackVersions({
  source,
  projectId,
  onInstall,
}: {
  source: ModpackSource;
  projectId: string;
  onInstall: (versionId: string) => void;
}) {
  const versions = useModpackVersions(source, projectId);

  if (versions.isPending) {
    return (
      <div className={modpackVersionsStyles.state}>
        <Spinner />
      </div>
    );
  }
  if (versions.isError) {
    return (
      <EmptyState
        icon={<TriangleAlertIcon />}
        title="Could not load versions"
        description={friendlyError(messageOf(versions.error))}
      />
    );
  }
  if (versions.data.length === 0) {
    return (
      <EmptyState
        icon={<LayersIcon />}
        title="No versions yet"
        description="This pack has no versions to install."
      />
    );
  }
  return <VersionTable versions={versions.data} onInstall={onInstall} />;
}

function VersionTable({
  versions,
  onInstall,
}: {
  versions: readonly ModpackVersionItem[];
  onInstall: (versionId: string) => void;
}) {
  const viewportRef = useRef<HTMLDivElement>(null);
  // One clock per table, so every row's "… ago" counts from the same moment.
  const [now] = useState(() => Date.now());
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLTableRowElement>({
    count: versions.length,
    getScrollElement: () => viewportRef.current,
    estimateSize: () => ROW_ESTIMATE_PX,
    getItemKey: (index) => versions[index]?.id ?? index,
    overscan: 6,
  });

  return (
    <ScrollArea
      viewportRef={viewportRef}
      className={modpackVersionsStyles.scrollArea}
    >
      <table
        aria-label="Versions"
        aria-rowcount={versions.length + 1}
        className={modpackVersionsStyles.table}
      >
        <thead className={modpackVersionsStyles.head}>
          <tr aria-rowindex={1} className={modpackVersionsStyles.columns}>
            <th scope="col" className={modpackVersionsStyles.headCell}>
              Name
            </th>
            <th scope="col" className={modpackVersionsStyles.headCell}>
              Game versions
            </th>
            <th scope="col" className={modpackVersionsStyles.headCell}>
              Loaders
            </th>
            <th scope="col" className={modpackVersionsStyles.headCell}>
              Published
            </th>
            <th scope="col" className={modpackVersionsStyles.headCell}>
              Downloads
            </th>
            <th scope="col" className={modpackVersionsStyles.headCell}>
              <span className={modpackVersionsStyles.srOnly}>Actions</span>
            </th>
          </tr>
        </thead>
        <tbody
          className={modpackVersionsStyles.body}
          // Rows are absolutely placed; only the visible window is rendered.
          style={{ height: virtualizer.getTotalSize() }}
        >
          {virtualizer.getVirtualItems().map((row) => {
            const version = versions[row.index];
            if (!version) return null;
            const published = formatTimeAgo(version.publishedAt, now);
            return (
              <tr
                key={row.key}
                ref={virtualizer.measureElement}
                data-index={row.index}
                aria-rowindex={row.index + 2}
                className={cn(
                  modpackVersionsStyles.columns,
                  modpackVersionsStyles.row,
                )}
                style={{ transform: `translateY(${row.start}px)` }}
              >
                <td className={modpackVersionsStyles.nameCell}>
                  <Badge
                    data-version-type={version.versionType}
                    className={
                      modpackVersionsStyles.typeTones[version.versionType]
                    }
                  >
                    {TYPE_LABELS[version.versionType]}
                  </Badge>
                  <span className={modpackVersionsStyles.name}>
                    {version.name}
                  </span>
                </td>
                <td
                  className={modpackVersionsStyles.muted}
                  title={version.gameVersions.join(", ")}
                >
                  {version.gameVersions.join(", ")}
                </td>
                <td className={modpackVersionsStyles.loaders}>
                  {version.loaders.map((loader) => (
                    <LoaderBadge key={loader} loader={loader} />
                  ))}
                </td>
                <td className={modpackVersionsStyles.muted}>
                  {published && (
                    <time
                      dateTime={version.publishedAt ?? undefined}
                      title={formatUpdatedAt(version.publishedAt) ?? undefined}
                    >
                      {published}
                    </time>
                  )}
                </td>
                <td className={modpackVersionsStyles.number}>
                  {formatDownloads(version.downloads)}
                </td>
                <td className={modpackVersionsStyles.action}>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    aria-label={`Install ${version.name}`}
                    onClick={() => onInstall(version.id)}
                  >
                    <DownloadIcon aria-hidden />
                    Install
                  </Button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </ScrollArea>
  );
}
