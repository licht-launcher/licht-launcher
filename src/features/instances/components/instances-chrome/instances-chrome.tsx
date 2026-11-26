import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { PlusIcon } from "lucide-react";
import { type ReactNode, useEffect, useMemo, useRef } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { getLastPlayedFolder } from "@/api/commands";
import { LauncherNavigation } from "@/components/title-bar/launcher-navigation";
import { TitleBar } from "@/components/title-bar/title-bar";
import { Button } from "@/components/ui/button";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { lastDiscoverSearch } from "@/features/discover/lib/last-search";
import { DownloadsPopover } from "@/features/downloads/components/downloads-popover";
import { STORAGE_LOADING_PREVIEW_EVENT } from "@/features/downloads/hooks/use-storage-loading-preview";
import type {
  InstanceSettingsAction,
  InstanceSettingsStatus,
} from "@/features/instance-settings/model/window-events";
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
import { useInstanceSettingsBridge } from "@/features/instances/hooks/use-instance-settings-bridge";
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
import { CreateInstanceMenu } from "@/features/new-instance/components/create-instance-menu";
import { CreateInstanceDialog } from "@/features/new-instance/create-instance-dialog";
import { uniqueName } from "@/features/new-instance/model/create-state";
import { PlayerAccountButton } from "@/features/settings/components/player-account-button";
import { APP_CODENAME, APP_VERSION } from "@/lib/app-release";
import { reportInterfaceError } from "@/lib/error-reporting";
import { cn } from "@/lib/utils";
import type { HomeSearch } from "@/routes/_app/index";

import { instancesChromeStyles } from "./instances-chrome.styles";

export function InstancesChrome({ children }: { children: ReactNode }) {
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
  const instanceParam = homeSearch?.instance ?? null;

  const { instances, username } = useInstancesData();
  const { selectedFolder, setSelectedFolder, selected } = useInstanceSelection(
    instances.data,
    instanceParam,
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
    if (
      pathname !== "/" ||
      instances.data === undefined ||
      selectedFolder === null
    ) {
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
    pathname,
    instances.data,
    selectedFolder,
    instanceParam,
    homeSearch?.prepare,
    navigate,
  ]);
  const dialogs = useInstanceDialogs();
  const discoverActive =
    pathname === "/discover" || pathname.startsWith("/discover/");
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    let active = true;
    let unsubscribe: (() => void) | undefined;
    listen(STORAGE_LOADING_PREVIEW_EVENT, () => {
      if (active) void navigate({ to: "/downloads" });
    })
      .then((stop) => {
        if (active) unsubscribe = stop;
        else stop();
      })
      .catch((error: unknown) => reportInterfaceError("promise", error));
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [navigate]);
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

  const { busy, rename, duplicate, remove, openFolder, changeBase } =
    useInstanceMutations({
      beginInstall,
      endInstall,
      clearActionFor: dialogs.clearActionFor,
    });

  // Duplicating needs no questions: the copy is named like the original plus
  // a number, and it can be renamed afterwards.
  function numberedCopy(folder: string, name: string) {
    return {
      folder,
      name: uniqueName(
        name,
        (instances.data ?? []).map((instance) => instance.name),
      ),
    };
  }

  const instanceSettingsStatus: InstanceSettingsStatus = {
    busy,
    versionBusy: changeBase.isPending,
    // Instances whose game or a download for them is running.
    activeFolders: (instances.data ?? [])
      .filter(
        (instance) =>
          playPhase(instance.folder) !== "idle" ||
          isInstalling(instance.folder),
      )
      .map((instance) => instance.folder),
  };
  useInstanceSettingsBridge(
    instanceSettingsStatus,
    (action: InstanceSettingsAction) => {
      switch (action.kind) {
        case "rename":
          return rename.mutateAsync({
            folder: action.folder,
            name: action.name,
            fromSettingsWindow: true,
          });
        case "changeBase":
          return changeBase.mutateAsync({
            folder: action.folder,
            base: action.base,
            fromSettingsWindow: true,
          });
        case "openFolder":
          return openFolder.mutateAsync({
            folder: action.folder,
            fromSettingsWindow: true,
          });
        case "duplicate": {
          const instance = instances.data?.find(
            (item) => item.folder === action.folder,
          );
          if (!instance) {
            return Promise.reject(new Error("instance not found"));
          }
          return duplicate.mutateAsync({
            ...numberedCopy(instance.folder, instance.name),
            fromSettingsWindow: true,
          });
        }
        case "delete":
          return remove.mutateAsync({
            folder: action.folder,
            fromSettingsWindow: true,
          });
      }
    },
  );

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
      openCreate: dialogs.openCreate,
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
      dialogs.openCreate,
    ],
  );

  const content = (
    <div className={instancesChromeStyles.root}>
      <div className={instancesChromeStyles.frame}>
        <div className={instancesChromeStyles.content}>
          <ChromeContext.Provider value={context}>
            {children}
          </ChromeContext.Provider>
        </div>
      </div>
    </div>
  );

  return (
    <>
      <div className={instancesChromeStyles.sidebar}>
        <TitleBar
          actions={
            <PlayerAccountButton
              onAddAccount={() => dialogs.openGlobalSettings("account")}
            />
          }
        >
          <LauncherNavigation
            libraryActive={pathname === "/"}
            onOpenLibrary={() => {
              if (pathname === "/") return;
              void navigate({
                to: "/",
                search:
                  selectedFolder === null ? {} : { instance: selectedFolder },
              });
            }}
            discoverActive={discoverActive}
            onOpenDiscover={() => {
              if (pathname === "/discover") return;
              void navigate({ to: "/discover", search: lastDiscoverSearch() });
            }}
            onOpenSettings={dialogs.openGlobalSettings}
          />
        </TitleBar>
        {pathname === "/downloads" || discoverActive ? (
          content
        ) : (
          <ResizablePanelGroup
            id={SIDEBAR_LAYOUT_ID}
            orientation="horizontal"
            className={instancesChromeStyles.sidebarPanel}
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
                selectedFolder={pathname === "/" ? selectedFolder : null}
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
                      openFolder.mutate({ folder: instance.folder });
                      break;
                    case "rename":
                      dialogs.openRename(instance.folder, instance.name);
                      break;
                    case "duplicate":
                      duplicate.mutate(
                        numberedCopy(instance.folder, instance.name),
                      );
                      break;
                    case "delete":
                      dialogs.openDelete(instance.folder, instance.name);
                      break;
                  }
                }}
                onSelect={(folder) => {
                  if (pathname === "/" && folder === selectedFolder) {
                    return;
                  }
                  setSelectedFolder(folder);
                  void navigate({
                    to: "/",
                    search: { instance: folder },
                  });
                }}
              />
            </ResizablePanel>
            <ResizableHandle
              aria-label="Resize sidebar"
              className={instancesChromeStyles.resizeHandle}
            />
            <ResizablePanel id={MAIN_PANEL_ID} minSize={480}>
              {content}
            </ResizablePanel>
          </ResizablePanelGroup>
        )}
        <footer
          role="contentinfo"
          aria-label="Download status"
          className={instancesChromeStyles.footer}
        >
          <div className={instancesChromeStyles.footerGrid}>
            <div className={instancesChromeStyles.footerStart}>
              <CreateInstanceMenu onScratch={dialogs.openCreate} side="top">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className={instancesChromeStyles.footerAction}
                >
                  <PlusIcon
                    aria-hidden
                    className={instancesChromeStyles.footerIcon}
                  />
                  Create instance
                </Button>
              </CreateInstanceMenu>
            </div>
            <div className={instancesChromeStyles.footerCenter}>
              <DownloadsPopover
                side="top"
                showLabel
                triggerClassName={instancesChromeStyles.footerAction}
                onOpen={() => void navigate({ to: "/downloads" })}
              />
            </div>
            <div className={instancesChromeStyles.footerEnd}>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className={cn(
                  instancesChromeStyles.footerAction,
                  instancesChromeStyles.footerActionShrink,
                )}
                onClick={() => dialogs.openGlobalSettings("updater")}
              >
                <span className={instancesChromeStyles.footerLabel}>
                  Early Release {APP_VERSION} · {APP_CODENAME}
                </span>
              </Button>
            </div>
          </div>
        </footer>
      </div>

      <CreateInstanceDialog
        open={dialogs.createOpen}
        onOpenChange={dialogs.setCreateOpen}
      />
      <InstanceDialogs
        busy={busy}
        mutations={{ rename, remove }}
        action={dialogs.action}
        draftName={dialogs.draftName}
        onActionOpenChange={(open) => {
          if (!open) {
            dialogs.clearAction();
          }
        }}
        onDraftNameChange={dialogs.setDraftName}
      />
    </>
  );
}
