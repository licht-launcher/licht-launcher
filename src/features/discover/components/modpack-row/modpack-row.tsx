import { Link } from "@tanstack/react-router";
import { DownloadIcon, PackageIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import { formatDownloads } from "@/features/discover/lib/format";
import type { ModpackListItem } from "@/lib/generated/ModpackListItem";

import { modpackRowStyles } from "./modpack-row.styles";

/** One search result; the whole row links to the pack's store page. */
export function ModpackRow({ modpack }: { modpack: ModpackListItem }) {
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
      </span>
      <span className={modpackRowStyles.meta}>
        <Badge>{SOURCE_LABELS[modpack.source]}</Badge>
        <span className={modpackRowStyles.downloads}>
          <DownloadIcon aria-hidden />
          <span>
            {formatDownloads(modpack.downloads)}
            <span className={modpackRowStyles.srOnly}> downloads</span>
          </span>
        </span>
      </span>
    </Link>
  );
}
