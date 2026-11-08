import { useNavigate } from "@tanstack/react-router";
import { PlusIcon } from "lucide-react";
import { useEffect, useRef } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { HomePanel } from "@/features/instances/components/home-panel";
import { useInstancesChrome } from "@/features/instances/components/instances-chrome-context";
import { CreateInstanceMenu } from "@/features/new-instance/components/create-instance-menu";
import { friendlyError, messageOf } from "@/lib/errors";
import { Route } from "@/routes/_app/index";

import { instancesPageStyles } from "./instances-page.styles";

export function InstancesPage() {
  const navigate = useNavigate();
  const { prepare } = Route.useSearch();
  const preparedRef = useRef<string | null>(null);
  const {
    instances,
    selectedFolder,
    selected,
    setSelectedFolder,
    busy,
    playBlocked,
    playPhase,
    isInstalling,
    downloadJobs,
    progressForVersion,
    speedForVersion,
    logsByFolder,
    clearFolderLog,
    playInstance,
    prepareInstance,
    stopInstance,
    openConfigure,
    openCreate,
  } = useInstancesChrome();

  useEffect(() => {
    if (prepare === undefined || instances.data === undefined) {
      return;
    }
    if (preparedRef.current === prepare) {
      return;
    }
    const instance = instances.data.find((entry) => entry.folder === prepare);
    if (instance === undefined) {
      return;
    }
    preparedRef.current = prepare;
    setSelectedFolder(prepare);
    void navigate({
      to: "/",
      search: { instance: prepare },
      replace: true,
    });
    void prepareInstance(instance).catch((error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    });
  }, [prepare, instances.data, navigate, prepareInstance, setSelectedFolder]);

  const phase = selected ? playPhase(selected.folder) : "idle";
  const installing = selected !== null && isInstalling(selected.folder);
  const job = downloadJobs.find((job) => job.instanceFolder === selectedFolder);
  const progress =
    (installing || job !== undefined) && selected
      ? (job?.progress ?? progressForVersion(selected.versionId))
      : null;
  const speed =
    installing && selected && job === undefined
      ? speedForVersion(selected.versionId)
      : 0;

  if (instances.isSuccess && instances.data.length === 0) {
    return (
      <section
        aria-labelledby="instances-welcome-title"
        className={instancesPageStyles.root}
      >
        <div className={instancesPageStyles.iconWell}>
          <PlusIcon aria-hidden className={instancesPageStyles.icon} />
        </div>
        <div className={instancesPageStyles.text}>
          <h1
            id="instances-welcome-title"
            className={instancesPageStyles.title}
          >
            Create your first instance
          </h1>
          <p className={instancesPageStyles.description}>
            Choose a Minecraft version or install a modpack to start playing.
          </p>
        </div>
        <CreateInstanceMenu onScratch={openCreate}>
          <Button type="button">
            <PlusIcon data-icon="inline-start" aria-hidden />
            Create instance
          </Button>
        </CreateInstanceMenu>
      </section>
    );
  }

  return (
    <HomePanel
      instance={selected}
      busy={busy}
      playDisabled={playBlocked}
      phase={phase}
      installing={installing}
      downloadJob={job}
      progress={progress}
      speedBytesPerSecond={speed}
      lines={
        selectedFolder === null ? [] : (logsByFolder[selectedFolder] ?? [])
      }
      onPlay={() => {
        if (selected) {
          void playInstance(selected);
        }
      }}
      onStop={() => {
        if (selected) {
          void stopInstance(selected.folder);
        }
      }}
      onConfigure={() => {
        if (selected) {
          openConfigure(selected.folder);
        }
      }}
      onClearLog={() => {
        if (selectedFolder !== null) {
          clearFolderLog(selectedFolder);
        }
      }}
    />
  );
}
