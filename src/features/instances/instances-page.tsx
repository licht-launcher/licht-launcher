import { useNavigate } from "@tanstack/react-router";
import { PlusIcon } from "lucide-react";
import { useEffect, useRef } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { HomePanel } from "@/features/instances/components/home-panel";
import { useInstancesChrome } from "@/features/instances/components/instances-chrome-context";
import { friendlyError, messageOf } from "@/lib/errors";
import { Route } from "@/routes/_app/index";

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
        className="flex min-h-0 flex-1 flex-col items-center justify-center gap-6 overflow-auto p-8 text-center"
      >
        <div className="flex size-16 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-primary">
          <PlusIcon aria-hidden className="size-8" />
        </div>
        <div className="max-w-md space-y-2">
          <h1
            id="instances-welcome-title"
            className="text-2xl font-semibold tracking-tight"
          >
            Create your first instance
          </h1>
          <p className="text-sm text-muted-foreground">
            Choose a Minecraft version or install a modpack to start playing.
          </p>
        </div>
        <Button
          type="button"
          onClick={() => {
            void navigate({
              to: "/instances/new",
              search: { step: "source" },
            });
          }}
        >
          <PlusIcon data-icon="inline-start" aria-hidden />
          Create instance
        </Button>
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
