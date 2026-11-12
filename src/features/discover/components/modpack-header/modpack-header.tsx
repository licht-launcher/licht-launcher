import {
  CalendarClockIcon,
  DownloadIcon,
  ExternalLinkIcon,
  HeartIcon,
  PackageIcon,
} from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { CategoryBadge } from "@/features/discover/components/category-badge";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import {
  formatDownloads,
  formatTimeAgo,
  formatUpdatedAt,
} from "@/features/discover/lib/format";
import { openLink } from "@/features/discover/lib/open-link";
import type { ModpackDetails } from "@/lib/generated/ModpackDetails";

import { modpackHeaderStyles } from "./modpack-header.styles";

/** Name, stats and categories of a pack, with its install and source actions. */
export function ModpackHeader({
  modpack,
  onInstall,
}: {
  modpack: ModpackDetails;
  onInstall: () => void;
}) {
  const [now] = useState(() => Date.now());
  const updated = formatTimeAgo(modpack.updatedAt, now);
  const sourceLabel = SOURCE_LABELS[modpack.source];
  const projectUrl = modpack.projectUrl;

  return (
    <header className={modpackHeaderStyles.root}>
      {modpack.iconUrl ? (
        <img
          src={modpack.iconUrl}
          alt=""
          className={modpackHeaderStyles.icon}
        />
      ) : (
        <span aria-hidden className={modpackHeaderStyles.iconFallback}>
          <PackageIcon />
        </span>
      )}
      <div className={modpackHeaderStyles.heading}>
        <div className={modpackHeaderStyles.titleLine}>
          <h1 className={modpackHeaderStyles.title}>{modpack.name}</h1>
          {modpack.author && (
            <span className={modpackHeaderStyles.author}>
              by {modpack.author}
            </span>
          )}
        </div>
        <p className={modpackHeaderStyles.summary}>{modpack.summary}</p>
        {modpack.categories.length > 0 && (
          <ul aria-label="Categories" className={modpackHeaderStyles.tags}>
            {modpack.categories.map((category) => (
              <li key={category}>
                <CategoryBadge category={category} />
              </li>
            ))}
          </ul>
        )}
        <div className={modpackHeaderStyles.stats}>
          <span className={modpackHeaderStyles.stat}>
            <DownloadIcon aria-hidden />
            {formatDownloads(modpack.downloads)} downloads
          </span>
          {modpack.followers !== null && (
            <span className={modpackHeaderStyles.stat}>
              <HeartIcon aria-hidden />
              {formatDownloads(modpack.followers)} followers
            </span>
          )}
          {updated && (
            <span className={modpackHeaderStyles.stat}>
              <CalendarClockIcon aria-hidden />
              <time
                dateTime={modpack.updatedAt ?? undefined}
                title={formatUpdatedAt(modpack.updatedAt) ?? undefined}
              >
                Updated {updated}
              </time>
            </span>
          )}
        </div>
      </div>
      <div className={modpackHeaderStyles.actions}>
        <Button type="button" onClick={onInstall}>
          <DownloadIcon aria-hidden />
          Install
        </Button>
        {projectUrl && (
          <Button
            type="button"
            variant="outline"
            onClick={() => openLink(projectUrl)}
          >
            <ExternalLinkIcon aria-hidden />
            Open on {sourceLabel}
          </Button>
        )}
      </div>
    </header>
  );
}
