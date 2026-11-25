import {
  BlocksIcon,
  EraserIcon,
  FolderIcon,
  FolderOpenIcon,
  SquareTerminalIcon,
} from "lucide-react";
import { useState } from "react";
import { openInstanceFolder } from "@/api/commands";
import { Button } from "@/components/ui/button";
import { PageTransition } from "@/components/ui/page-transition";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import type { LogLevelSelection } from "@/features/instances/lib/log-level";
import { modpackOrigin } from "@/features/instances/lib/modpack-content";
import type { ConsoleLine } from "@/features/instances/model/console-line";
import {
  ConsoleLogLevelFilter,
  ConsoleSection,
} from "@/features/instances/sections/console-section";
import {
  type ContentFilter,
  ContentSection,
} from "@/features/instances/sections/content-section";
import {
  FilesBreadcrumb,
  FilesSection,
} from "@/features/instances/sections/files-section";
import { PlaySection } from "@/features/instances/sections/play-section";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

import { homePanelStyles } from "./home-panel.styles";

const INSTANCE_TABS = ["content", "files", "console"];

export function HomePanel({
  instance,
  busy,
  playDisabled,
  phase,
  installing,
  downloadJob,
  progress,
  speedBytesPerSecond,
  lines,
  onPlay,
  onStop,
  onConfigure,
  onClearLog,
}: {
  instance: InstanceEntry | null;
  busy: boolean;
  playDisabled: boolean;
  phase: PlayPhase;
  installing: boolean;
  downloadJob?: DownloadJobSnapshot;
  progress: DownloadProgress | null;
  speedBytesPerSecond: number;
  lines: ConsoleLine[];
  onPlay: () => void;
  onStop: () => void;
  onConfigure: () => void;
  onClearLog: () => void;
}) {
  const [tab, setTab] = useState("content");
  const [tabDirection, setTabDirection] = useState<"left" | "right">("left");
  const [contentFilter, setContentFilter] = useState<ContentFilter>("all");
  const [contentSearch, setContentSearch] = useState("");
  const [logLevels, setLogLevels] = useState<LogLevelSelection>(
    () => new Set(),
  );
  const [filesPath, setFilesPath] = useState("");
  const [pathFolder, setPathFolder] = useState<string | null>(null);
  const folder = instance?.folder ?? null;
  if (pathFolder !== folder) {
    setPathFolder(folder);
    setFilesPath("");
    setTab("content");
  }

  const modpack = modpackOrigin(instance?.installation?.source);
  return (
    <div className={homePanelStyles.root}>
      <div className={homePanelStyles.header}>
        <PlaySection
          instance={instance}
          busy={busy}
          playDisabled={playDisabled}
          phase={phase}
          installing={installing}
          downloadJob={downloadJob}
          progress={progress}
          speedBytesPerSecond={speedBytesPerSecond}
          onPlay={onPlay}
          onStop={onStop}
          onConfigure={onConfigure}
        />
      </div>
      <Tabs
        value={tab}
        onValueChange={(next) => {
          const nextIndex = INSTANCE_TABS.indexOf(next);
          if (nextIndex === -1 || next === tab) return;
          setTabDirection(
            nextIndex > INSTANCE_TABS.indexOf(tab) ? "left" : "right",
          );
          setTab(next);
        }}
        className={homePanelStyles.tabs}
      >
        <div className={homePanelStyles.toolbar}>
          <TabsList variant="sliding" className={homePanelStyles.tabsList}>
            <TabsTrigger
              value="content"
              className={homePanelStyles.tabsTrigger}
            >
              <BlocksIcon aria-hidden className={homePanelStyles.tabIcon} />
              Content
            </TabsTrigger>
            <TabsTrigger value="files" className={homePanelStyles.tabsTrigger}>
              <FolderIcon aria-hidden className={homePanelStyles.tabIcon} />
              Files
            </TabsTrigger>
            <TabsTrigger
              value="console"
              className={homePanelStyles.tabsTrigger}
            >
              <SquareTerminalIcon
                aria-hidden
                className={homePanelStyles.tabIcon}
              />
              Console
            </TabsTrigger>
          </TabsList>
          <div className={homePanelStyles.tabActions}>
            {tab === "console" ? (
              <>
                <Button
                  type="button"
                  variant="outline"
                  disabled={lines.length === 0}
                  aria-label="Clear console"
                  onClick={onClearLog}
                >
                  <EraserIcon data-icon="inline-start" aria-hidden />
                  Clear
                </Button>
                <ConsoleLogLevelFilter
                  selected={logLevels}
                  onSelectedChange={setLogLevels}
                />
              </>
            ) : tab === "files" ? (
              <>
                <FilesBreadcrumb
                  folder={folder}
                  relativePath={filesPath}
                  onRelativePathChange={setFilesPath}
                />
                <Button
                  type="button"
                  variant="outline"
                  className={homePanelStyles.header}
                  disabled={instance === null}
                  onClick={() => {
                    if (instance !== null) {
                      void openInstanceFolder(instance.folder);
                    }
                  }}
                >
                  <FolderOpenIcon data-icon="inline-start" aria-hidden />
                  Open folder
                </Button>
              </>
            ) : null}
          </div>
        </div>
        <TabsContent value="files" className={homePanelStyles.tabContent}>
          <PageTransition direction={tabDirection}>
            <FilesSection
              folder={folder}
              relativePath={filesPath}
              onRelativePathChange={setFilesPath}
            />
          </PageTransition>
        </TabsContent>
        <TabsContent value="console" className={homePanelStyles.tabContent}>
          <PageTransition direction={tabDirection}>
            <ConsoleSection lines={lines} selected={logLevels} />
          </PageTransition>
        </TabsContent>
        <TabsContent value="content" className={homePanelStyles.tabContent}>
          <PageTransition direction={tabDirection}>
            <ContentSection
              folder={folder}
              filter={contentFilter}
              onFilterChange={setContentFilter}
              search={contentSearch}
              onSearchChange={setContentSearch}
              modpack={modpack}
              linkedModpack={instance?.installation?.modpack ?? null}
              iconPath={instance?.iconPath ?? null}
            />
          </PageTransition>
        </TabsContent>
      </Tabs>
    </div>
  );
}
