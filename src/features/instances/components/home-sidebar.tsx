import { useRouter, useRouterState } from "@tanstack/react-router";
import { cn } from "cn";
import {
  ArrowDownAZIcon,
  ArrowLeftIcon,
  ArrowRightIcon,
  CompassIcon,
  DownloadIcon,
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
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { InstanceIcon } from "@/features/instances/components/instance-icon";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import {
  type InstanceSort,
  sortInstances,
} from "@/features/instances/lib/sort-instances";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const sidebarOrbWrapClassName =
  "inline-flex transition-transform duration-200 ease-out hover:-translate-y-2";

const sidebarOrbBaseClassName =
  "size-10 rounded-full shadow-[0_4px_12px_0_rgb(0_0_0/0.28)] [&_svg:not([class*='size-'])]:size-5";

const sidebarOrbClassName = cn(
  sidebarOrbBaseClassName,
  "transition-shadow duration-200 ease-out hover:shadow-[0_6px_16px_0_rgb(0_0_0/0.38)] disabled:pointer-events-auto disabled:shadow-[0_4px_12px_0_rgb(0_0_0/0.28)]",
);

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

function SidebarHistoryNav() {
  const router = useRouter();
  useRouterState({
    select: (state) =>
      `${String(state.location.state.__TSR_index)}:${state.location.href}`,
  });
  const canGoBack = router.history.canGoBack();
  const canGoForward =
    router.history.location.state.__TSR_index < router.history.length - 1;

  return (
    <div className="flex shrink-0 items-center gap-0.5">
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label="Back"
        className="text-muted-foreground"
        disabled={!canGoBack}
        onClick={() => {
          router.history.back();
        }}
      >
        <ArrowLeftIcon aria-hidden />
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label="Forward"
        className="text-muted-foreground"
        disabled={!canGoForward}
        onClick={() => {
          router.history.forward();
        }}
      >
        <ArrowRightIcon aria-hidden />
      </Button>
    </div>
  );
}

export function HomeSidebar({
  instances,
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
  const ordered = sortInstances(instances, sort);

  return (
    <div className="flex h-full w-full min-w-0 flex-col overflow-hidden bg-black/20">
      <div className="flex h-9 shrink-0 items-center justify-between gap-2 px-2">
        <div
          data-tauri-drag-region
          className="flex min-h-9 min-w-0 flex-1 items-center"
        >
          {showTitleband ? (
            <span
              data-tauri-drag-region
              className="ml-px border border-transparent px-1 font-brand text-xs font-medium tracking-widest text-muted-foreground uppercase select-none"
            >
              Licht Launcher
            </span>
          ) : null}
        </div>
        <SidebarHistoryNav />
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-6 px-2 pt-1 pb-10">
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
              <p className="border border-transparent px-1 text-sm text-muted-foreground">
                Loading…
              </p>
            ) : null}
            {!loading && instances.length === 0 ? (
              <p className="border border-transparent px-1 text-sm text-muted-foreground">
                No instances yet.
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
                      <li key={instance.folder} className="min-w-0 max-w-full">
                        <Button
                          type="button"
                          variant={selected ? "secondary" : "ghost"}
                          className={cn(
                            "grid w-full min-w-0 max-w-full grid-cols-[auto_minmax(0,1fr)_auto] gap-2 overflow-hidden px-1 text-[0.8rem]",
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
                    );
                  })}
                </ul>
              </ScrollArea>
            ) : null}
          </div>

          <TooltipProvider delayDuration={300}>
            <div className="mt-auto flex items-center justify-center gap-1.5 pt-2">
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className={sidebarOrbWrapClassName}>
                    <Button
                      type="button"
                      variant="secondary"
                      size="icon-lg"
                      className={sidebarOrbClassName}
                      aria-label="New instance"
                      disabled={busy}
                      onClick={onCreate}
                    >
                      <PlusIcon aria-hidden />
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent side="top" align="center" sideOffset={2}>
                  New instance
                </TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="inline-flex">
                    <Button
                      type="button"
                      variant="secondary"
                      size="icon-lg"
                      className={sidebarOrbBaseClassName}
                      aria-label="Downloads"
                      disabled
                    >
                      <DownloadIcon aria-hidden />
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent side="top" align="center" sideOffset={2}>
                  Downloads
                </TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="inline-flex">
                    <Button
                      type="button"
                      variant="secondary"
                      size="icon-lg"
                      className={sidebarOrbBaseClassName}
                      aria-label="Discover"
                      disabled
                    >
                      <CompassIcon aria-hidden />
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent side="top" align="center" sideOffset={2}>
                  Discover
                </TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className={sidebarOrbWrapClassName}>
                    <Button
                      type="button"
                      variant="secondary"
                      size="icon-lg"
                      className={sidebarOrbClassName}
                      aria-label="Settings"
                      onClick={onOpenSettings}
                    >
                      <SettingsIcon aria-hidden />
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent side="top" align="center" sideOffset={2}>
                  Settings
                </TooltipContent>
              </Tooltip>
            </div>
          </TooltipProvider>
        </nav>
      </div>
    </div>
  );
}
