import { Link } from "@tanstack/react-router";
import {
  ArrowLeftIcon,
  CalendarIcon,
  DownloadIcon,
  ExternalLinkIcon,
  PackageIcon,
  TriangleAlertIcon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { ModpackDescription } from "@/features/discover/components/modpack-description";
import { ModpackInstallPanel } from "@/features/discover/components/modpack-install-panel";
import { useModpack } from "@/features/discover/hooks/use-modpacks";
import { SOURCE_LABELS } from "@/features/discover/lib/discover-search";
import {
  formatDownloads,
  formatUpdatedAt,
} from "@/features/discover/lib/format";
import { openLink } from "@/features/discover/lib/open-link";
import { friendlyError, messageOf } from "@/lib/errors";
import type { ModpackDetails } from "@/lib/generated/ModpackDetails";
import type { ModpackSource } from "@/lib/generated/ModpackSource";

import { modpackPageStyles } from "./modpack-page.styles";

/** Store page for one modpack: details on the left, install on the right. */
export function ModpackPage({
  source,
  projectId,
}: {
  source: ModpackSource;
  projectId: string;
}) {
  const modpack = useModpack(source, projectId);

  return (
    <div className={modpackPageStyles.root}>
      <div className={modpackPageStyles.toolbar}>
        <Button asChild variant="ghost" size="sm">
          <Link to="/discover">
            <ArrowLeftIcon aria-hidden />
            Discover
          </Link>
        </Button>
      </div>
      {modpack.isPending ? (
        <div className={modpackPageStyles.state}>
          <Spinner />
        </div>
      ) : modpack.isError ? (
        <EmptyState
          icon={<TriangleAlertIcon />}
          title="Could not load this modpack"
          description={friendlyError(messageOf(modpack.error))}
        />
      ) : (
        <div className={modpackPageStyles.layout}>
          <ScrollArea className={modpackPageStyles.main}>
            <div className={modpackPageStyles.mainContent}>
              <ModpackHeader modpack={modpack.data} />
              {modpack.data.gallery.length > 0 && (
                <section aria-labelledby="modpack-gallery">
                  <h2
                    id="modpack-gallery"
                    className={modpackPageStyles.sectionTitle}
                  >
                    Gallery
                  </h2>
                  <ul className={modpackPageStyles.gallery}>
                    {modpack.data.gallery.map((url) => (
                      <li key={url}>
                        <img
                          src={url}
                          alt=""
                          loading="lazy"
                          className={modpackPageStyles.galleryImage}
                        />
                      </li>
                    ))}
                  </ul>
                </section>
              )}
              <section aria-labelledby="modpack-description">
                <h2
                  id="modpack-description"
                  className={modpackPageStyles.sectionTitle}
                >
                  Description
                </h2>
                <ModpackDescription html={modpack.data.descriptionHtml} />
              </section>
            </div>
          </ScrollArea>
          <aside className={modpackPageStyles.aside}>
            <ScrollArea className={modpackPageStyles.asideScroll}>
              <ModpackInstallPanel modpack={modpack.data} />
            </ScrollArea>
          </aside>
        </div>
      )}
    </div>
  );
}

function ModpackHeader({ modpack }: { modpack: ModpackDetails }) {
  const updatedAt = formatUpdatedAt(modpack.updatedAt);
  const sourceLabel = SOURCE_LABELS[modpack.source];
  const projectUrl = modpack.projectUrl;

  return (
    <header className={modpackPageStyles.header}>
      {modpack.iconUrl ? (
        <img src={modpack.iconUrl} alt="" className={modpackPageStyles.icon} />
      ) : (
        <span aria-hidden className={modpackPageStyles.iconFallback}>
          <PackageIcon />
        </span>
      )}
      <div className={modpackPageStyles.heading}>
        <h1 className={modpackPageStyles.title}>{modpack.name}</h1>
        {modpack.author && (
          <p className={modpackPageStyles.author}>by {modpack.author}</p>
        )}
        <p className={modpackPageStyles.summary}>{modpack.summary}</p>
        <div className={modpackPageStyles.meta}>
          <Badge>{sourceLabel}</Badge>
          <span className={modpackPageStyles.metaItem}>
            <DownloadIcon aria-hidden />
            {formatDownloads(modpack.downloads)} downloads
          </span>
          {updatedAt && (
            <span className={modpackPageStyles.metaItem}>
              <CalendarIcon aria-hidden />
              Updated {updatedAt}
            </span>
          )}
        </div>
        {modpack.categories.length > 0 && (
          <ul aria-label="Categories" className={modpackPageStyles.categories}>
            {modpack.categories.map((category) => (
              <li key={category}>
                <Badge>{category}</Badge>
              </li>
            ))}
          </ul>
        )}
        {projectUrl && (
          <div className={modpackPageStyles.actions}>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => openLink(projectUrl)}
            >
              <ExternalLinkIcon aria-hidden />
              Open on {sourceLabel}
            </Button>
          </div>
        )}
      </div>
    </header>
  );
}
