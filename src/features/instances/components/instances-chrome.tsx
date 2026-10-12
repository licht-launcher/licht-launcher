import type { UseQueryResult } from "@tanstack/react-query";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import {
  createContext,
  type ReactNode,
  useContext,
  useMemo,
  useRef,
} from "react";
import { TitleBar } from "@/components/title-bar/title-bar";
import { HomeSidebar } from "@/features/instances/components/home-sidebar";
import { InstanceDialogs } from "@/features/instances/components/instance-dialogs";
import { useGameEvents } from "@/features/instances/hooks/use-game-events";
import { useInstanceDialogs } from "@/features/instances/hooks/use-instance-dialogs";
import { useInstanceMutations } from "@/features/instances/hooks/use-instance-mutations";
import { useInstanceSelection } from "@/features/instances/hooks/use-instance-selection";
import { useInstancesData } from "@/features/instances/hooks/use-instances-data";
import {
  type PlayPhase,
  usePlaySession,
} from "@/features/instances/hooks/use-play-session";
import { useTitleBarDecorations } from "@/hooks/use-title-bar-decorations";
import { getLastPlayedFolder } from "@/lib/commands";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

export type InstancesChromeContext = {
  versions: UseQueryResult<VersionSummary[], Error>;
  instances: UseQueryResult<InstanceEntry[], Error>;
  username: string;
  selectedFolder: string | null;
  setSelectedFolder: (folder: string | null) => void;
  selected: InstanceEntry | null;
  busy: boolean;
  playBlocked: boolean;
  playPhase: (folder: string) => PlayPhase;
  isInstalling: (folder: string) => boolean;
  progressForVersion: ReturnType<typeof useGameEvents>["progressForVersion"];
  logsByFolder: ReturnType<typeof useGameEvents>["logsByFolder"];
  lastPlayedFolder: string | null;
  playInstance: ReturnType<typeof usePlaySession>["playInstance"];
  prepareInstance: ReturnType<typeof usePlaySession>["prepareInstance"];
  stopInstance: ReturnType<typeof usePlaySession>["stopInstance"];
  openConfigure: (folder: string) => void;
};

const ChromeContext = createContext<InstancesChromeContext | null>(null);

export function useInstancesChrome(): InstancesChromeContext {
  const value = useContext(ChromeContext);
  if (value === null) {
    throw new Error("useInstancesChrome must be used inside InstancesChrome");
  }
  return value;
}

export function InstancesChrome({ children }: { children: ReactNode }) {
  const { disableCustomTitleBar } = useTitleBarDecorations();
  const navigate = useNavigate();
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const onWizard = pathname === "/instances/new";

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

  const lastPlayed = useQuery({
    queryKey: ["last-played"],
    queryFn: getLastPlayedFolder,
    retry: false,
  });

  const {
    isInstalling,
    beginInstall,
    endInstall,
    playBlocked,
    playPhase,
    markStarted,
    playInstance,
    prepareInstance,
    stopInstance,
  } = usePlaySession({
    username,
    clearVersionProgress,
    clearFolderLog,
  });
  markStartedRef.current = markStarted;

  const { busy, rename, duplicate, remove, openFolder, changeVersion } =
    useInstanceMutations({
      beginInstall,
      endInstall,
      setConfiguringFolder: dialogs.setConfiguringFolder,
      clearAction: dialogs.clearAction,
    });

  const configuring =
    instances.data?.find(
      (instance) => instance.folder === dialogs.configuringFolder,
    ) ?? null;

  const context = useMemo<InstancesChromeContext>(
    () => ({
      versions,
      instances,
      username,
      selectedFolder,
      setSelectedFolder,
      selected,
      busy,
      playBlocked,
      playPhase,
      isInstalling,
      progressForVersion,
      logsByFolder,
      lastPlayedFolder: lastPlayed.data ?? null,
      playInstance,
      prepareInstance,
      stopInstance,
      openConfigure: dialogs.openConfigure,
    }),
    [
      versions,
      instances,
      username,
      selectedFolder,
      setSelectedFolder,
      selected,
      busy,
      playBlocked,
      playPhase,
      isInstalling,
      progressForVersion,
      logsByFolder,
      lastPlayed.data,
      playInstance,
      prepareInstance,
      stopInstance,
      dialogs.openConfigure,
    ],
  );

  return (
    <>
      <div className="flex h-full w-full overflow-hidden bg-popover text-popover-foreground">
        <HomeSidebar
          instances={instances.data ?? []}
          lastPlayedFolder={lastPlayed.data ?? null}
          selectedFolder={onWizard ? null : selectedFolder}
          busy={busy}
          loading={instances.isPending}
          playPhase={playPhase}
          onSelect={(folder) => {
            setSelectedFolder(folder);
            if (onWizard) {
              void navigate({ to: "/", search: {} });
            }
          }}
          onCreate={() => {
            void navigate({ to: "/instances/new" });
          }}
          onOpenSettings={() => dialogs.openGlobalSettings("account")}
        />
        <div className="flex min-w-0 min-h-0 flex-1 flex-col overflow-hidden">
          {disableCustomTitleBar ? null : <TitleBar />}
          <div className="min-h-0 min-w-0 flex-1 overflow-hidden">
            <ChromeContext.Provider value={context}>
              {children}
            </ChromeContext.Provider>
          </div>
        </div>
      </div>

      <InstanceDialogs
        configuring={configuring}
        versions={versions.data ?? []}
        busy={busy}
        mutations={{
          rename,
          duplicate,
          remove,
          openFolder,
          changeVersion,
        }}
        configuringOpen={dialogs.configuringFolder !== null}
        action={dialogs.action}
        draftName={dialogs.draftName}
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
