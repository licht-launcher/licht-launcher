import { cn } from "cn";
import {
  ArrowDownAZIcon,
  ArrowDownZAIcon,
  HistoryIcon,
  Loader2Icon,
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
import {
  InstanceContextMenu,
  type InstanceMenuAction,
} from "@/features/instances/components/instance-context-menu";
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
  selectedFolder,
  busy,
  loading,
  playPhase,
  isInstalling,
  playBlocked,
  onInstanceAction,
  onSelect,
}: {
  instances: InstanceEntry[];
  selectedFolder: string | null;
  busy: boolean;
  loading: boolean;
  playPhase: (folder: string) => PlayPhase;
  isInstalling: (folder: string) => boolean;
  playBlocked: boolean;
  onInstanceAction: (
    action: InstanceMenuAction,
    instance: InstanceEntry,
  ) => void;
  onSelect: (folder: string) => void;
}) {
  const [sort, setSort] = useState<InstanceSort>("lastPlayed");
  const ordered = sortInstances(instances, sort);

  return (
    <div className="flex h-full w-full min-w-0 flex-col overflow-hidden">
      <div className="flex min-h-0 flex-1 flex-col px-2 py-2">
        <nav
          aria-label="Instances"
          className="flex min-h-0 flex-1 flex-col gap-2"
        >
          <div className="flex min-h-0 flex-1 flex-col">
            <div className="ml-px flex items-center gap-2 border border-transparent px-1">
              <h2 className="min-w-0 flex-1 font-brand text-xs font-medium leading-none text-muted-foreground">
                Instances
              </h2>
              <Select
                value={sort}
                onValueChange={(value) => {
                  if (
                    value === "lastPlayed" ||
                    value === "name" ||
                    value === "nameDesc"
                  ) {
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
                  position="popper"
                  side="bottom"
                  align="end"
                  className="z-100 font-brand text-xs font-medium"
                >
                  <SelectItem value="lastPlayed" className="text-xs">
                    <SortLabel icon={HistoryIcon} label="Last played" />
                  </SelectItem>
                  <SelectItem value="name" className="text-xs">
                    <SortLabel icon={ArrowDownAZIcon} label="A–Z" />
                  </SelectItem>
                  <SelectItem value="nameDesc" className="text-xs">
                    <SortLabel icon={ArrowDownZAIcon} label="Z–A" />
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            {loading ? (
              <p className="border border-transparent px-1 text-sm text-muted-foreground">
                Loading…
              </p>
            ) : null}
            {!loading && ordered.length > 0 ? (
              <ScrollArea className="min-h-0 flex-1">
                <ul className="flex w-full min-w-0 flex-col px-px">
                  {ordered.map((instance) => {
                    const selected = instance.folder === selectedFolder;
                    const phase = playPhase(instance.folder);
                    const active = phase === "starting" || phase === "running";
                    return (
                      <InstanceContextMenu
                        key={instance.folder}
                        instance={instance}
                        phase={phase}
                        installing={isInstalling(instance.folder)}
                        busy={busy}
                        playBlocked={playBlocked}
                        onAction={onInstanceAction}
                      >
                        <li className="min-w-0 max-w-full">
                          <Button
                            type="button"
                            variant={selected ? "secondary" : "ghost"}
                            className={cn(
                              "grid w-full min-w-0 max-w-full grid-cols-[auto_minmax(0,1fr)_auto] gap-2 overflow-hidden px-1 text-[0.8rem]",
                              selected &&
                                "border-foreground/10 bg-secondary/35 hover:border-foreground/20 hover:bg-secondary/50",
                              !selected && "text-muted-foreground/70",
                            )}
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
                            <InstanceIcon iconPath={instance.iconPath} />
                            <span
                              className="min-w-0 overflow-hidden text-left text-ellipsis whitespace-nowrap"
                              title={instance.name}
                            >
                              {instance.name}
                            </span>
                            {active ? (
                              <span
                                aria-hidden
                                className="inline-flex min-w-0 items-center gap-1.5 overflow-hidden text-ellipsis whitespace-nowrap text-xs font-medium text-success"
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
                          </Button>
                        </li>
                      </InstanceContextMenu>
                    );
                  })}
                </ul>
              </ScrollArea>
            ) : null}
          </div>
        </nav>
      </div>
    </div>
  );
}
