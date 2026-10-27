import { useIsFetching, useQueryClient } from "@tanstack/react-query";
import {
  BlocksIcon,
  EraserIcon,
  FolderIcon,
  FolderOpenIcon,
  ImageIcon,
  RefreshCwIcon,
  SearchIcon,
  SparklesIcon,
  SquareTerminalIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PageTransition } from "@/components/ui/page-transition";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import { instanceContentQuery } from "@/features/instances/lib/content-query";
import type { LogLevelSelection } from "@/features/instances/lib/log-level";
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
import { openInstanceFolder } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

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
  const queryClient = useQueryClient();
  const folder = instance?.folder ?? null;
  const refreshingContent =
    useIsFetching({ queryKey: ["instance-content", folder] }) > 0;
  if (pathFolder !== folder) {
    setPathFolder(folder);
    setFilesPath("");
    setTab("content");
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden p-4">
      <div className="shrink-0">
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
        className="mt-3 flex min-h-0 flex-1 flex-col gap-2"
      >
        <div className="flex shrink-0 items-center gap-2">
          <TabsList variant="sliding" className="w-fit shrink-0">
            <TabsTrigger
              value="content"
              className="cursor-pointer gap-1.5 px-3"
            >
              <BlocksIcon aria-hidden className="size-4 shrink-0" />
              Content
            </TabsTrigger>
            <TabsTrigger value="files" className="cursor-pointer gap-1.5 px-3">
              <FolderIcon aria-hidden className="size-4 shrink-0" />
              Files
            </TabsTrigger>
            <TabsTrigger
              value="console"
              className="cursor-pointer gap-1.5 px-3"
            >
              <SquareTerminalIcon aria-hidden className="size-4 shrink-0" />
              Console
            </TabsTrigger>
          </TabsList>
          {tab === "console" ? (
            <div className="ml-auto flex shrink-0 items-center gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
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
            </div>
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
                size="sm"
                className="shrink-0"
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
          ) : tab === "content" ? (
            <>
              <Input
                aria-label="Search content"
                className="ml-auto w-52 font-medium"
                placeholder="Search content"
                value={contentSearch}
                startIcon={<SearchIcon aria-hidden />}
                onChange={(event) => setContentSearch(event.target.value)}
              />
              <Select
                value={contentFilter}
                onValueChange={(value) =>
                  setContentFilter(value as ContentFilter)
                }
              >
                <SelectTrigger
                  aria-label="Filter content"
                  className="w-36 font-medium"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">
                    <BlocksIcon aria-hidden />
                    All content
                  </SelectItem>
                  <SelectItem value="mods">
                    <BlocksIcon aria-hidden />
                    Mods
                  </SelectItem>
                  <SelectItem value="resourcePacks">
                    <ImageIcon aria-hidden />
                    Resource Packs
                  </SelectItem>
                  <SelectItem value="shaderPacks">
                    <SparklesIcon aria-hidden />
                    Shader Packs
                  </SelectItem>
                </SelectContent>
              </Select>
              <Button
                type="button"
                variant="outline"
                size="icon"
                aria-label="Refresh content"
                disabled={folder === null || refreshingContent}
                onClick={() => {
                  if (folder === null) return;
                  void queryClient
                    .fetchQuery({
                      ...instanceContentQuery(queryClient, folder, true),
                      staleTime: 0,
                    })
                    .catch((error: unknown) =>
                      toast.error(friendlyError(messageOf(error))),
                    );
                }}
              >
                <RefreshCwIcon
                  aria-hidden
                  className={refreshingContent ? "animate-spin" : undefined}
                />
              </Button>
            </>
          ) : null}
        </div>
        <TabsContent
          value="files"
          className="mt-0 flex min-h-0 flex-1 flex-col outline-none"
        >
          <PageTransition direction={tabDirection}>
            <FilesSection
              folder={folder}
              relativePath={filesPath}
              onRelativePathChange={setFilesPath}
            />
          </PageTransition>
        </TabsContent>
        <TabsContent
          value="console"
          className="mt-0 flex min-h-0 flex-1 flex-col outline-none"
        >
          <PageTransition direction={tabDirection}>
            <ConsoleSection lines={lines} selected={logLevels} />
          </PageTransition>
        </TabsContent>
        <TabsContent
          value="content"
          className="mt-0 flex min-h-0 flex-1 flex-col outline-none"
        >
          <PageTransition direction={tabDirection}>
            <ContentSection
              folder={folder}
              filter={contentFilter}
              search={contentSearch}
            />
          </PageTransition>
        </TabsContent>
      </Tabs>
    </div>
  );
}
