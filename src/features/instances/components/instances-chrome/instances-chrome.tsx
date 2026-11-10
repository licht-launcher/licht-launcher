import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouterState } from "@tanstack/react-router";
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
import { DownloadsPopover } from "@/features/downloads/components/downloads-popover";
import { STORAGE_LOADING_PREVIEW_EVENT } from "@/features/downloads/hooks/use-storage-loading-preview";
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
import { CreateInstanceMenu } from "@/features/new-instance/components/create-instance-menu";
import { CreateInstanceDialog } from "@/features/new-instance/create-instance-dialog";
import { uniqueName } from "@/features/new-instance/model/create-state";
import { PlayerAccountButton } from "@/features/settings/components/player-account-button";
import { useCustomTitleBar } from "@/hooks/use-custom-title-bar";
import { APP_CODENAME, APP_VERSION } from "@/lib/app-release";
import { cn } from "@/lib/utils";
import type { HomeSearch } from "@/routes/_app/index";

import { instancesChromeStyles } from "./instances-chrome.styles";

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
  const discoverActive = pathname.startsWith("/discover");
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    const preview = () => {
      dialogs.setSettingsOpen(false);
      void navigate({ to: "/downloads" });
    };
    window.addEventListener(STORAGE_LOADING_PREVIEW_EVENT, preview);
    return () =>
      window.removeEventListener(STORAGE_LOADING_PREVIEW_EVENT, preview);
  }, [dialogs.setSettingsOpen, navigate]);
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

  // Duplicating needs no questions: the copy is named like the original plus
  // a number, and it can be renamed afterwards.
  function duplicateNumbered(folder: string, name: string) {
    duplicate.mutate({
      folder,
      name: uniqueName(
        name,
        (instances.data ?? []).map((instance) => instance.name),
      ),
    });
  }

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
          windowControls={customTitleBar}
          actions={
            <PlayerAccountButton
              onAddAccount={() => dialogs.openGlobalSettings("account")}
            />
          }
        >
          <LauncherNavigation
            libraryActive={
              pathname === "/" &&
              !(dialogs.settingsOpen && dialogs.settingsSection === "account")
            }
            onOpenLibrary={() => {
              dialogs.setSettingsOpen(false);
              if (pathname === "/") return;
              void navigate({
                to: "/",
                search:
                  selectedFolder === null ? {} : { instance: selectedFolder },
              });
            }}
            discoverActive={discoverActive}
            onOpenDiscover={() => {
              dialogs.setSettingsOpen(false);
              if (pathname === "/discover") return;
              void navigate({ to: "/discover" });
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
                      openFolder.mutate(instance.folder);
                      break;
                    case "rename":
                      dialogs.openRename(instance.folder, instance.name);
                      break;
                    case "duplicate":
                      duplicateNumbered(instance.folder, instance.name);
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
                onOpen={() => {
                  dialogs.setSettingsOpen(false);
                  void navigate({ to: "/downloads" });
                }}
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
        configuring={configuring}
        busy={busy}
        mutations={{
          rename,
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
          duplicateNumbered(configuring.folder, configuring.name);
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
