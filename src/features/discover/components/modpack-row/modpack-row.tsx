import { Link } from "@tanstack/react-router";
import {
  CalendarClockIcon,
  DownloadIcon,
  PackageIcon,
  StoreIcon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { LoaderBadge } from "@/features/discover/components/loader-badge";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import { CATEGORY_BY_VALUE } from "@/features/discover/lib/filter-options";
import {
  formatDownloads,
  formatTimeAgo,
  formatUpdatedAt,
} from "@/features/discover/lib/format";
import type { ModpackListItem } from "@/lib/generated/ModpackListItem";

import { modpackRowStyles } from "./modpack-row.styles";

/** Categories shown per row; the pack's page lists them all. */
const MAX_ROW_CATEGORIES = 3;

/**
 * One search result; the whole row links to the pack's store page.
 * `now` is shared by every row so relative times agree.
 */
export function ModpackRow({
  modpack,
  now,
}: {
  modpack: ModpackListItem;
  now: number;
}) {
  const updated = formatTimeAgo(modpack.updatedAt, now);
  const categories = modpack.categories
    .slice(0, MAX_ROW_CATEGORIES)
    .flatMap((value) => CATEGORY_BY_VALUE.get(value) ?? []);

  return (
    <Link
      to="/discover/$source/$projectId"
      params={{ source: modpack.source, projectId: modpack.id }}
      className={modpackRowStyles.link}
    >
      {modpack.iconUrl ? (
        <img
          src={modpack.iconUrl}
          alt=""
          loading="lazy"
          className={modpackRowStyles.icon}
        />
      ) : (
        <span aria-hidden className={modpackRowStyles.iconFallback}>
          <PackageIcon />
        </span>
      )}
      <span className={modpackRowStyles.text}>
        <span className={modpackRowStyles.title}>
          <span className={modpackRowStyles.name}>{modpack.name}</span>
          {modpack.author && (
            <span className={modpackRowStyles.author}>by {modpack.author}</span>
          )}
        </span>
        <span className={modpackRowStyles.description}>
          {modpack.description}
        </span>
        {(modpack.loaders.length > 0 || categories.length > 0) && (
          <span className={modpackRowStyles.tags}>
            {modpack.loaders.map((loader) => (
              <LoaderBadge key={loader} loader={loader} />
            ))}
            {categories.map((category) => (
              <Badge key={category.value}>
                <category.icon aria-hidden />
                {category.label}
              </Badge>
            ))}
          </span>
        )}
      </span>
      <span className={modpackRowStyles.meta}>
        <span className={modpackRowStyles.stat}>
          <DownloadIcon aria-hidden />
          <span>
            {formatDownloads(modpack.downloads)}
            <span className={modpackRowStyles.srOnly}> downloads</span>
          </span>
        </span>
        {updated && (
          <span className={modpackRowStyles.stat}>
            <CalendarClockIcon aria-hidden />
            <time
              dateTime={modpack.updatedAt ?? undefined}
              title={formatUpdatedAt(modpack.updatedAt) ?? undefined}
            >
              Updated {updated}
            </time>
          </span>
        )}
        {/* The same pack can be listed by both stores; this tells them apart. */}
        <span className={modpackRowStyles.stat}>
          <StoreIcon aria-hidden />
          {SOURCE_LABELS[modpack.source]}
        </span>
      </span>
    </Link>
  );
}
