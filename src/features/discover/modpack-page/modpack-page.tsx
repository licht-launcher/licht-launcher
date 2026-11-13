import {
  FileTextIcon,
  ImagesIcon,
  LayersIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { EmptyState } from "@/components/ui/empty-state";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ModpackDescription } from "@/features/discover/components/modpack-description";
import { ModpackGallery } from "@/features/discover/components/modpack-gallery";
import { ModpackHeader } from "@/features/discover/components/modpack-header";
import { ModpackReinstallDialog } from "@/features/discover/components/modpack-reinstall-dialog";
import { ModpackSidebar } from "@/features/discover/components/modpack-sidebar";
import { ModpackVersions } from "@/features/discover/components/modpack-versions";
import { useInstallModpack } from "@/features/discover/hooks/use-install-modpack";
import {
  useModpack,
  useModpackVersions,
} from "@/features/discover/hooks/use-modpacks";
import { defaultVersionId } from "@/features/discover/lib/default-version";
import { hasInstalledModpack } from "@/features/discover/lib/installed-modpack";
import { useInstanceList } from "@/features/new-instance/hooks/use-instance-names";
import { uniqueName } from "@/features/new-instance/model/create-state";
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
  const versions = useModpackVersions(modpack.source, modpack.id);
  const instanceList = useInstanceList();
  const instances = instanceList.data ?? [];
  const install = useInstallModpack(modpack);
  const hasGallery = modpack.gallery.length > 0;
  const latestVersionId = defaultVersionId(versions.data ?? []);
  const [confirmVersionId, setConfirmVersionId] = useState<string | null>(null);
  const takenNames = instances.map((instance) => instance.name);
  const alreadyInstalled = hasInstalledModpack(
    instances,
    modpack.source,
    modpack.id,
  );

  function installVersion(versionId: string) {
    if (install.isPending) {
      return;
    }
    install.mutate({
      name: uniqueName(modpack.name, takenNames),
      versionId,
    });
  }

  function requestInstall(versionId: string) {
    if (install.isPending || instanceList.isPending) {
      return;
    }
    if (alreadyInstalled) {
      setConfirmVersionId(versionId);
      return;
    }
    installVersion(versionId);
  }

  function confirmInstall() {
    if (confirmVersionId === null) {
      return;
    }
    const versionId = confirmVersionId;
    setConfirmVersionId(null);
    installVersion(versionId);
  }

  function installLatest() {
    if (versions.isError) {
      toast.error(friendlyError(messageOf(versions.error)));
      return;
    }
    if (latestVersionId === null) {
      toast.error("This pack has no versions to install.");
      return;
    }
    requestInstall(latestVersionId);
  }

  return (
    <>
      <ModpackHeader
        modpack={modpack}
        disabled={
          versions.isPending || instanceList.isPending || install.isPending
        }
        pending={install.isPending}
        onInstall={installLatest}
      />
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
              installing={instanceList.isPending || install.isPending}
              onInstall={requestInstall}
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
      <ModpackReinstallDialog
        open={confirmVersionId !== null}
        pending={install.isPending}
        onOpenChange={(open) => {
          if (!open) {
            setConfirmVersionId(null);
          }
        }}
        onConfirm={confirmInstall}
      />
    </>
  );
}
