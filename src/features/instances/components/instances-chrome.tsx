import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { type ReactNode, useEffect, useMemo, useRef } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { TitleBar } from "@/components/title-bar/title-bar";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { HomeSidebar } from "@/features/instances/components/home-sidebar";
import { InstanceDialogs } from "@/features/instances/components/instance-dialogs";
import {
  ChromeContext,
  type InstancesChromeContext,
} from "@/features/instances/components/instances-chrome-context";
import { useGameEvents } from "@/features/instances/hooks/use-game-events";
import { useInstanceDialogs } from "@/features/instances/hooks/use-instance-dialogs";
import { useInstanceMutations } from "@/features/instances/hooks/use-instance-mutations";
import { useInstanceSelection } from "@/features/instances/hooks/use-instance-selection";
import { useInstancesData } from "@/features/instances/hooks/use-instances-data";
import { usePlaySession } from "@/features/instances/hooks/use-play-session";
import { instanceContentQuery } from "@/features/instances/lib/content-query";
import {
  MAIN_PANEL_ID,
  SIDEBAR_DEFAULT_SIZE_PX,
  SIDEBAR_LAYOUT_ID,
  SIDEBAR_MAX_SIZE_PX,
  SIDEBAR_MIN_SIZE_PX,
  SIDEBAR_PANEL_ID,
  SIDEBAR_PANEL_IDS,
  sidebarLayoutStorage,
} from "@/features/instances/lib/sidebar-layout";
import { useCustomTitleBar } from "@/hooks/use-custom-title-bar";
import { getLastPlayedFolder } from "@/lib/commands";
import type { HomeSearch } from "@/routes/_app/index";

export function InstancesChrome({ children }: { children: ReactNode }) {
  const customTitleBar = useCustomTitleBar().data ?? true;
  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: SIDEBAR_LAYOUT_ID,
    panelIds: [...SIDEBAR_PANEL_IDS],
    storage: sidebarLayoutStorage,
    onlySaveAfterUserInteractions: true,
  });
  const navigate = useNavigate();
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const homeSearch = useRouterState({
    select: (state): HomeSearch | null => {
      if (state.location.pathname !== "/") {
        return null;
      }
      return state.location.search as HomeSearch;
    },
  });
  const onWizard = pathname === "/instances/new";
  const instanceParam = homeSearch?.instance ?? null;

  const { instances, username } = useInstancesData();
  const { selectedFolder, setSelectedFolder, selected } = useInstanceSelection(
    instances.data,
    onWizard ? null : instanceParam,
  );
  const queryClient = useQueryClient();

  useEffect(() => {
    if (selectedFolder === null) {
      return;
    }
    void queryClient.prefetchQuery(
      instanceContentQuery(queryClient, selectedFolder),
    );
  }, [queryClient, selectedFolder]);

  useEffect(() => {
    if (onWizard || instances.data === undefined || selectedFolder === null) {
      return;
    }
    if (homeSearch?.prepare !== undefined) {
      return;
    }
    // A valid URL selection takes precedence while the selection hook catches up.
    if (
      instanceParam !== null &&
      instances.data.some((instance) => instance.folder === instanceParam)
    ) {
      return;
    }
    void navigate({
      to: "/",
      search: { instance: selectedFolder },
      replace: true,
    });
  }, [
    onWizard,
    instances.data,
    selectedFolder,
    instanceParam,
    homeSearch?.prepare,
    navigate,
  ]);
  const dialogs = useInstanceDialogs();
  const markStartedRef = useRef<(folder: string) => void>(() => {});
  const {
    progressForVersion,
    speedForVersion,
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
    downloadJobs,
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
      instances,
      username,
      selectedFolder,
      setSelectedFolder,
      selected,
      busy,
      playBlocked,
      playPhase,
      isInstalling,
      downloadJobs,
      progressForVersion,
      speedForVersion,
      logsByFolder,
      clearFolderLog,
      lastPlayedFolder: lastPlayed.data ?? null,
      playInstance,
      prepareInstance,
      stopInstance,
      openConfigure: dialogs.openConfigure,
    }),
    [
      instances,
      username,
      selectedFolder,
      setSelectedFolder,
      selected,
      busy,
      playBlocked,
      playPhase,
      isInstalling,
      downloadJobs,
      progressForVersion,
      speedForVersion,
      logsByFolder,
      clearFolderLog,
      lastPlayed.data,
      playInstance,
      prepareInstance,
      stopInstance,
      dialogs.openConfigure,
    ],
  );

  return (
    <>
      <ResizablePanelGroup
        id={SIDEBAR_LAYOUT_ID}
        orientation="horizontal"
        className="min-h-0 h-full w-full flex-1 overflow-hidden bg-popover text-popover-foreground"
        defaultLayout={defaultLayout}
        onLayoutChanged={onLayoutChanged}
        disableCursor
      >
        <ResizablePanel
          id={SIDEBAR_PANEL_ID}
          defaultSize={SIDEBAR_DEFAULT_SIZE_PX}
          minSize={SIDEBAR_MIN_SIZE_PX}
          maxSize={SIDEBAR_MAX_SIZE_PX}
        >
          <HomeSidebar
            instances={instances.data ?? []}
            selectedFolder={onWizard ? null : selectedFolder}
            busy={busy}
            loading={instances.isPending}
            playPhase={playPhase}
            isInstalling={isInstalling}
            playBlocked={playBlocked}
            onInstanceAction={(action, instance) => {
              switch (action) {
                case "play":
                  void playInstance(instance);
                  break;
                case "stop":
                  void stopInstance(instance.folder);
                  break;
                case "configure":
                  dialogs.openConfigure(instance.folder);
                  break;
                case "open-folder":
                  openFolder.mutate(instance.folder);
                  break;
                case "rename":
                  dialogs.openRename(instance.folder, instance.name);
                  break;
                case "duplicate":
                  dialogs.openDuplicate(instance.folder, instance.name);
                  break;
                case "delete":
                  dialogs.openDelete(instance.folder, instance.name);
                  break;
              }
            }}
            showTitleband={customTitleBar}
            onSelect={(folder) => {
              if (!onWizard && folder === selectedFolder) {
                return;
              }
              setSelectedFolder(folder);
              void navigate({
                to: "/",
                search: { instance: folder },
              });
            }}
            onCreate={() => {
              void navigate({
                to: "/instances/new",
                search: { step: "source" },
              });
            }}
            onOpenSettings={() => dialogs.openGlobalSettings("account")}
          />
        </ResizablePanel>
        <ResizableHandle aria-label="Resize sidebar" />
        <ResizablePanel id={MAIN_PANEL_ID} minSize={480}>
          <div className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden bg-muted/30">
            {customTitleBar ? <TitleBar /> : null}
            <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
              <ChromeContext.Provider value={context}>
                {children}
              </ChromeContext.Provider>
            </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>

      <InstanceDialogs
        configuring={configuring}
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
