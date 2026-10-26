import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useRouterState } from "@tanstack/react-router";
import { PlusIcon } from "lucide-react";
import { type ReactNode, useEffect, useMemo, useRef } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { LauncherNavigation } from "@/components/title-bar/launcher-navigation";
import { TitleBar } from "@/components/title-bar/title-bar";
import { Button } from "@/components/ui/button";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { DownloadsPopover } from "@/features/downloads/components/downloads-popover";
import {
  STORAGE_LOADING_PREVIEW_EVENT,
  STORAGE_LOADING_PREVIEW_QUERY_KEY,
} from "@/features/downloads/hooks/use-storage-loading-preview";
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
import { PlayerAccountButton } from "@/features/settings/components/player-account-button";
import { useCustomTitleBar } from "@/hooks/use-custom-title-bar";
import { getLastPlayedFolder } from "@/lib/commands";
import type { HomeSearch } from "@/routes/_app/index";

const footerActionClassName =
  "h-8 gap-2 rounded-md p-0 text-xs text-muted-foreground hover:bg-transparent hover:text-foreground dark:hover:bg-transparent aria-expanded:bg-transparent aria-expanded:text-foreground";

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
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    const preview = () => {
      queryClient.setQueryData(STORAGE_LOADING_PREVIEW_QUERY_KEY, Date.now());
      dialogs.setSettingsOpen(false);
      void navigate({ to: "/downloads" });
    };
    window.addEventListener(STORAGE_LOADING_PREVIEW_EVENT, preview);
    return () =>
      window.removeEventListener(STORAGE_LOADING_PREVIEW_EVENT, preview);
  }, [queryClient, dialogs.setSettingsOpen, navigate]);
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

  const content = (
    <div className="flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <div className="mx-2 mt-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border-t border-foreground/10 bg-popover">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-muted/30">
          <ChromeContext.Provider value={context}>
            {children}
          </ChromeContext.Provider>
        </div>
      </div>
    </div>
  );

  return (
    <>
      <div className="flex h-full min-h-0 w-full flex-1 flex-col overflow-hidden bg-home-sidebar text-popover-foreground">
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
            onOpenSettings={dialogs.openGlobalSettings}
          />
        </TitleBar>
        {pathname === "/downloads" ? (
          content
        ) : (
          <ResizablePanelGroup
            id={SIDEBAR_LAYOUT_ID}
            orientation="horizontal"
            className="min-h-0 w-full flex-1 overflow-hidden"
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
                      dialogs.openDuplicate(instance.folder, instance.name);
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
              className="w-0 bg-transparent hover:scale-x-100 hover:bg-transparent focus-visible:scale-x-100 focus-visible:bg-transparent active:scale-x-100 active:bg-transparent"
            />
            <ResizablePanel id={MAIN_PANEL_ID} minSize={480}>
              {content}
            </ResizablePanel>
          </ResizablePanelGroup>
        )}
        <footer
          role="contentinfo"
          aria-label="Download status"
          className="flex shrink-0 items-center justify-center px-3 pt-0.5 pb-1.5"
        >
          <div className="grid min-h-8 w-full grid-cols-4 items-center">
            <div className="flex items-center justify-start">
              <Button
                asChild
                variant="ghost"
                size="sm"
                className={footerActionClassName}
              >
                <Link to="/instances/new" search={{ step: "source" }}>
                  <PlusIcon aria-hidden className="size-3.5" />
                  Create instance
                </Link>
              </Button>
            </div>
            <div className="col-span-2 flex min-w-0 items-center justify-center">
              <DownloadsPopover
                side="top"
                showLabel
                triggerClassName={footerActionClassName}
                onOpen={() => {
                  dialogs.setSettingsOpen(false);
                  void navigate({ to: "/downloads" });
                }}
              />
            </div>
          </div>
        </footer>
      </div>

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
