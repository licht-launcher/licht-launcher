import { useRef } from "react";

import { HomePanel } from "@/features/instances/components/home-panel";
import { HomeSidebar } from "@/features/instances/components/home-sidebar";
import { InstanceDialogs } from "@/features/instances/components/instance-dialogs";
import { useGameEvents } from "@/features/instances/hooks/use-game-events";
import { useInstanceDialogs } from "@/features/instances/hooks/use-instance-dialogs";
import { useInstanceMutations } from "@/features/instances/hooks/use-instance-mutations";
import { useInstanceSelection } from "@/features/instances/hooks/use-instance-selection";
import { useInstancesData } from "@/features/instances/hooks/use-instances-data";
import { usePlaySession } from "@/features/instances/hooks/use-play-session";

export function InstancesPage() {
  const { versions, instances, username } = useInstancesData();
  const { selectedFolder, setSelectedFolder, selected } = useInstanceSelection(
    instances.data,
  );
  const dialogs = useInstanceDialogs();
  const markStartedRef = useRef<(folder: string) => void>(() => {});
  const {
    progressForVersion,
    clearVersionProgress,
    logsByFolder,
    clearFolderLog,
  } = useGameEvents((folder) => {
    markStartedRef.current(folder);
  });

  const {
    isInstalling,
    beginInstall,
    endInstall,
    playBlocked,
    playPhase,
    markStarted,
    playInstance,
    stopInstance,
  } = usePlaySession({
    versions: versions.data,
    username,
    clearVersionProgress,
    clearFolderLog,
  });
  markStartedRef.current = markStarted;

  const { busy, create, rename, duplicate, remove, openFolder, changeVersion } =
    useInstanceMutations({
      beginInstall,
      endInstall,
      setSelectedFolder,
      setConfiguringFolder: dialogs.setConfiguringFolder,
      clearAction: dialogs.clearAction,
    });

  const configuring =
    instances.data?.find(
      (instance) => instance.folder === dialogs.configuringFolder,
    ) ?? null;

  const phase = selected ? playPhase(selected.folder) : "idle";
  const installing = selected !== null && isInstalling(selected.folder);
  const progress =
    installing && selected ? progressForVersion(selected.versionId) : null;

  return (
    <>
      <div className="flex h-svh w-full overflow-hidden bg-popover text-popover-foreground">
        <HomeSidebar
          instances={instances.data ?? []}
          selectedFolder={selectedFolder}
          busy={busy}
          loading={instances.isPending}
          onSelect={setSelectedFolder}
          onCreate={() => dialogs.openCreate(versions.data ?? [])}
          onOpenSettings={() => dialogs.openGlobalSettings("account")}
        />
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
              dialogs.openConfigure(selected.folder);
            }
          }}
        />
      </div>

      <InstanceDialogs
        configuring={configuring}
        versions={versions.data ?? []}
        versionsPending={versions.isPending}
        busy={busy}
        mutations={{
          create,
          rename,
          duplicate,
          remove,
          openFolder,
          changeVersion,
        }}
        configuringOpen={dialogs.configuringFolder !== null}
        action={dialogs.action}
        draftName={dialogs.draftName}
        draftVersion={dialogs.draftVersion}
        includeSnapshots={dialogs.includeSnapshots}
        settingsOpen={dialogs.settingsOpen}
        settingsSection={dialogs.settingsSection}
        onSettingsOpenChange={dialogs.setSettingsOpen}
        onConfiguringOpenChange={(open) => {
          if (!open) {
            dialogs.setConfiguringFolder(null);
          }
        }}
        onActionOpenChange={(open) => {
          if (!open) {
            dialogs.clearAction();
          }
        }}
        onDraftNameChange={dialogs.setDraftName}
        onDraftVersionChange={dialogs.setDraftVersion}
        onIncludeSnapshotsChange={dialogs.setIncludeSnapshots}
        onOpenGlobalSettings={dialogs.openGlobalSettings}
        onOpenDuplicate={() => {
          if (!configuring) {
            return;
          }
          dialogs.openDuplicate(configuring.folder, configuring.name);
        }}
        onOpenDelete={() => {
          if (!configuring) {
            return;
          }
          dialogs.openDelete(configuring.folder, configuring.name);
        }}
      />
    </>
  );
}
