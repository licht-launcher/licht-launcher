import { Link } from "@tanstack/react-router";
import {
  ArrowLeftIcon,
  FileTextIcon,
  ImagesIcon,
  LayersIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ModpackDescription } from "@/features/discover/components/modpack-description";
import { ModpackGallery } from "@/features/discover/components/modpack-gallery";
import { ModpackHeader } from "@/features/discover/components/modpack-header";
import { ModpackInstallDialog } from "@/features/discover/components/modpack-install-dialog";
import { ModpackSidebar } from "@/features/discover/components/modpack-sidebar";
import { ModpackVersions } from "@/features/discover/components/modpack-versions";
import {
  useModpack,
  useModpackVersions,
} from "@/features/discover/hooks/use-modpacks";
import { friendlyError, messageOf } from "@/lib/errors";
import type { ModpackDetails } from "@/lib/generated/ModpackDetails";
import type { ModpackSource } from "@/lib/generated/ModpackSource";

import { modpackPageStyles } from "./modpack-page.styles";

/** Store page for one modpack: header, tabs on the left, sidebar on the right. */
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
        <ModpackContent modpack={modpack.data} />
      )}
    </div>
  );
}

function ModpackContent({ modpack }: { modpack: ModpackDetails }) {
  const [install, setInstall] = useState<{
    open: boolean;
    versionId: string | null;
  }>({ open: false, versionId: null });
  const versions = useModpackVersions(modpack.source, modpack.id);
  const hasGallery = modpack.gallery.length > 0;

  function openInstall(versionId: string | null) {
    setInstall({ open: true, versionId });
  }

  return (
    <>
      <ModpackHeader modpack={modpack} onInstall={() => openInstall(null)} />
      <div className={modpackPageStyles.layout}>
        <Tabs defaultValue="description" className={modpackPageStyles.tabs}>
          <TabsList variant="sliding" className={modpackPageStyles.tabsList}>
            <TabsTrigger
              value="description"
              className={modpackPageStyles.tabsTrigger}
            >
              <FileTextIcon aria-hidden />
              Description
            </TabsTrigger>
            {hasGallery && (
              <TabsTrigger
                value="gallery"
                className={modpackPageStyles.tabsTrigger}
              >
                <ImagesIcon aria-hidden />
                Gallery
              </TabsTrigger>
            )}
            <TabsTrigger
              value="versions"
              className={modpackPageStyles.tabsTrigger}
            >
              <LayersIcon aria-hidden />
              Versions
              {/* Keeps the count a separate word in the tab's name. */}
              {versions.data && " "}
              {versions.data && (
                <span className={modpackPageStyles.tabCount}>
                  {versions.data.length}
                </span>
              )}
            </TabsTrigger>
          </TabsList>
          <TabsContent
            value="description"
            className={modpackPageStyles.tabContent}
          >
            <ScrollArea className={modpackPageStyles.tabScroll}>
              <div className={modpackPageStyles.tabBody}>
                <ModpackDescription html={modpack.descriptionHtml} />
              </div>
            </ScrollArea>
          </TabsContent>
          {hasGallery && (
            <TabsContent
              value="gallery"
              className={modpackPageStyles.tabContent}
            >
              <ScrollArea className={modpackPageStyles.tabScroll}>
                <div className={modpackPageStyles.tabBody}>
                  <ModpackGallery images={modpack.gallery} />
                </div>
              </ScrollArea>
            </TabsContent>
          )}
          <TabsContent
            value="versions"
            className={modpackPageStyles.tabContent}
          >
            <ModpackVersions
              source={modpack.source}
              projectId={modpack.id}
              onInstall={openInstall}
            />
          </TabsContent>
        </Tabs>
        <aside
          aria-label="About this modpack"
          className={modpackPageStyles.aside}
        >
          <ScrollArea className={modpackPageStyles.asideScroll}>
            <ModpackSidebar modpack={modpack} />
          </ScrollArea>
        </aside>
      </div>
      <ModpackInstallDialog
        modpack={modpack}
        open={install.open}
        versionId={install.versionId}
        onOpenChange={(open) => setInstall((current) => ({ ...current, open }))}
      />
    </>
  );
}
