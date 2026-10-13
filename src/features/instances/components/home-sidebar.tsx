import {
  ArrowDownAZIcon,
  HistoryIcon,
  Loader2Icon,
  PlusIcon,
  SettingsIcon,
} from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import {
  type InstanceSort,
  sortInstances,
} from "@/features/instances/lib/sort-instances";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

function SortLabel({
  icon: Icon,
  label,
}: {
  icon: typeof HistoryIcon;
  label: string;
}) {
  return (
    <span className="flex items-center gap-1.5 font-brand text-xs font-medium leading-none">
      <Icon aria-hidden className="size-3.5 shrink-0" strokeWidth={2} />
      {label}
    </span>
  );
}

export function HomeSidebar({
  instances,
  lastPlayedFolder,
  selectedFolder,
  busy,
  loading,
  playPhase,
  showTitleband,
  onSelect,
  onCreate,
  onOpenSettings,
}: {
  instances: InstanceEntry[];
  lastPlayedFolder: string | null;
  selectedFolder: string | null;
  busy: boolean;
  loading: boolean;
  playPhase: (folder: string) => PlayPhase;
  showTitleband: boolean;
  onSelect: (folder: string) => void;
  onCreate: () => void;
  onOpenSettings: () => void;
}) {
  const [sort, setSort] = useState<InstanceSort>("lastPlayed");
  const ordered = sortInstances(instances, sort, lastPlayedFolder);

  return (
    <div className="flex h-full w-72 shrink-0 flex-col overflow-hidden border-r border-border bg-black/20">
      {showTitleband ? (
        <div
          data-tauri-drag-region
          className="flex h-9 shrink-0 items-center px-4"
        >
          <span
            data-tauri-drag-region
            className="font-brand text-xs font-medium tracking-widest text-muted-foreground uppercase select-none"
          >
            Licht Launcher
          </span>
        </div>
      ) : null}
      <div
        className={
          showTitleband
            ? "flex min-h-0 flex-1 flex-col gap-6 px-4 pt-4 pb-10"
            : "flex min-h-0 flex-1 flex-col gap-6 px-4 pt-6 pb-10"
        }
      >
        <nav
          aria-label="Instances"
          className="flex min-h-0 flex-1 flex-col gap-2"
        >
          <div className="flex items-center gap-2 px-2">
            <h2 className="min-w-0 flex-1 font-brand text-xs font-medium leading-none text-muted-foreground">
              Instances
            </h2>
            <Select
              value={sort}
              onValueChange={(value) => {
                if (value === "lastPlayed" || value === "name") {
                  setSort(value);
                }
              }}
            >
              <SelectTrigger
                size="sm"
                aria-label="Sort instances"
                className="h-auto w-auto max-w-40 shrink-0 gap-1 border-0 bg-transparent px-0 py-0 font-brand text-xs font-medium leading-none text-muted-foreground shadow-none dark:bg-transparent dark:hover:bg-transparent [&_svg:not([class*='size-'])]:size-3.5"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent
                align="end"
                className="font-brand text-xs font-medium"
              >
                <SelectItem value="lastPlayed" className="text-xs">
                  <SortLabel icon={HistoryIcon} label="Last played" />
                </SelectItem>
                <SelectItem value="name" className="text-xs">
                  <SortLabel icon={ArrowDownAZIcon} label="Name" />
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          {loading ? (
            <p className="px-2 text-sm text-muted-foreground">Loading…</p>
          ) : null}
          {!loading && instances.length === 0 ? (
            <p className="px-2 text-sm text-muted-foreground">
              No instances yet.
            </p>
          ) : null}
          {!loading && ordered.length > 0 ? (
            <ScrollArea className="min-h-0 flex-1">
              <ul className="flex w-full min-w-0 flex-col gap-1 pr-3">
                {ordered.map((instance) => {
                  const selected = instance.folder === selectedFolder;
                  const phase = playPhase(instance.folder);
                  const active = phase === "starting" || phase === "running";
                  return (
                    <li key={instance.folder} className="min-w-0">
                      <Button
                        type="button"
                        variant={selected ? "secondary" : "ghost"}
                        className="w-full min-w-0 max-w-full justify-start gap-2 overflow-hidden px-3"
                        aria-current={selected ? "page" : undefined}
                        aria-label={
                          active
                            ? `${instance.name} (${phase === "starting" ? "starting" : "running"})`
                            : undefined
                        }
                        disabled={busy}
                        onClick={() => {
                          onSelect(instance.folder);
                        }}
                      >
                        <span className="flex min-w-0 flex-1 items-center gap-2 overflow-hidden">
                          <InstanceIcon />
                          <span className="w-0 min-w-0 flex-1 truncate text-left">
                            {instance.name}
                          </span>
                          {active ? (
                            <span
                              aria-hidden
                              className="inline-flex shrink-0 items-center gap-1.5 text-xs font-medium text-success"
                            >
                              {phase === "starting" ? "Starting" : "Running"}
                              {phase === "starting" ? (
                                <Loader2Icon
                                  className="size-3.5 shrink-0 animate-spin"
                                  strokeWidth={2}
                                />
                              ) : (
                                <span className="relative flex size-2 shrink-0">
                                  <span className="absolute inline-flex size-full animate-ping rounded-full bg-success opacity-75" />
                                  <span className="relative inline-flex size-2 rounded-full bg-success" />
                                </span>
                              )}
                            </span>
                          ) : null}
                        </span>
                      </Button>
                    </li>
                  );
                })}
              </ul>
            </ScrollArea>
          ) : null}

          <Button
            type="button"
            variant="secondary"
            className="w-full justify-start px-3"
            disabled={busy}
            onClick={onCreate}
          >
            <PlusIcon data-icon="inline-start" aria-hidden />
            New instance
          </Button>

          <Button
            type="button"
            variant="ghost"
            className="w-full justify-start px-3"
            aria-label="Settings"
            onClick={onOpenSettings}
          >
            <SettingsIcon data-icon="inline-start" aria-hidden />
            Settings
          </Button>
        </nav>

        <p className="mt-auto px-3 text-xs leading-relaxed text-muted-foreground">
          Unofficial, not affiliated with Mojang Studios or Microsoft.
        </p>
      </div>
    </div>
  );
}
