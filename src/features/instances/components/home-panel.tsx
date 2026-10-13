import { FolderIcon, FolderOpenIcon, SquareTerminalIcon } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import type { LogLevelSelection } from "@/features/instances/lib/log-level";
import type { ConsoleLine } from "@/features/instances/model/console-line";
import {
  ConsoleLogLevelFilter,
  ConsoleSection,
} from "@/features/instances/sections/console-section";
import {
  FilesBreadcrumb,
  FilesSection,
} from "@/features/instances/sections/files-section";
import { PlaySection } from "@/features/instances/sections/play-section";
import { openInstanceFolder } from "@/lib/commands";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export function HomePanel({
  instance,
  busy,
  playDisabled,
  phase,
  installing,
  progress,
  lines,
  onPlay,
  onStop,
  onConfigure,
}: {
  instance: InstanceEntry | null;
  busy: boolean;
  playDisabled: boolean;
  phase: PlayPhase;
  installing: boolean;
  progress: DownloadProgress | null;
  lines: ConsoleLine[];
  onPlay: () => void;
  onStop: () => void;
  onConfigure: () => void;
}) {
  const [tab, setTab] = useState("console");
  const [logLevels, setLogLevels] = useState<LogLevelSelection>(
    () => new Set(),
  );
  const [filesPath, setFilesPath] = useState("");
  const [pathFolder, setPathFolder] = useState<string | null>(null);
  const folder = instance?.folder ?? null;
  if (pathFolder !== folder) {
    setPathFolder(folder);
    setFilesPath("");
    setTab("console");
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden px-8 pt-6 pr-12 pb-8">
      <div className="shrink-0">
        <PlaySection
          instance={instance}
          busy={busy}
          playDisabled={playDisabled}
          phase={phase}
          installing={installing}
          progress={progress}
          onPlay={onPlay}
          onStop={onStop}
          onConfigure={onConfigure}
        />
      </div>
      <Tabs
        value={tab}
        onValueChange={setTab}
        className="mt-6 flex min-h-0 flex-1 flex-col gap-2"
      >
        <div className="flex shrink-0 items-center gap-3">
          <TabsList className="w-fit shrink-0">
            <TabsTrigger value="console" className="gap-1.5 px-3">
              <SquareTerminalIcon aria-hidden className="size-4 shrink-0" />
              Console
            </TabsTrigger>
            <TabsTrigger value="files" className="gap-1.5 px-3">
              <FolderIcon aria-hidden className="size-4 shrink-0" />
              Files
            </TabsTrigger>
          </TabsList>
          {tab === "console" ? (
            <div className="ml-auto shrink-0">
              <ConsoleLogLevelFilter
                selected={logLevels}
                onSelectedChange={setLogLevels}
              />
            </div>
          ) : (
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
          )}
        </div>
        <TabsContent
          value="console"
          className="mt-0 flex min-h-0 flex-1 flex-col outline-none"
        >
          <ConsoleSection lines={lines} selected={logLevels} />
        </TabsContent>
        <TabsContent
          value="files"
          className="mt-0 flex min-h-0 flex-1 flex-col outline-none"
        >
          <FilesSection
            folder={folder}
            relativePath={filesPath}
            onRelativePathChange={setFilesPath}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
}
