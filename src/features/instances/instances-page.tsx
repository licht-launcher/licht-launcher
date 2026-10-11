import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { toast } from "sonner";

import { HomePanel } from "@/features/instances/components/home-panel";
import { useInstancesChrome } from "@/features/instances/components/instances-chrome";
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
    progressForVersion,
    logsByFolder,
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
    void navigate({ to: "/", search: {}, replace: true });
    void prepareInstance(instance).catch((error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    });
  }, [prepare, instances.data, navigate, prepareInstance, setSelectedFolder]);

  const phase = selected ? playPhase(selected.folder) : "idle";
  const installing = selected !== null && isInstalling(selected.folder);
  const progress =
    installing && selected ? progressForVersion(selected.versionId) : null;

  return (
    <HomePanel
      instance={selected}
      busy={busy}
      playDisabled={playBlocked}
      phase={phase}
      installing={installing}
      progress={progress}
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
    />
  );
}
