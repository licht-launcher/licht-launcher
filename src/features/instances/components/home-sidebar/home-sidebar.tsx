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

import { homeSidebarStyles } from "./home-sidebar.styles";

function SortLabel({
  icon: Icon,
  label,
}: {
  icon: typeof HistoryIcon;
  label: string;
}) {
  return (
    <span className={homeSidebarStyles.sortLabel}>
      <Icon
        aria-hidden
        className={homeSidebarStyles.sortIcon}
        strokeWidth={2}
      />
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
    <div className={homeSidebarStyles.root}>
      <div className={homeSidebarStyles.body}>
        <nav aria-label="Instances" className={homeSidebarStyles.nav}>
          <div className={homeSidebarStyles.section}>
            <div className={homeSidebarStyles.sectionHeader}>
              <h2 className={homeSidebarStyles.sectionTitle}>Instances</h2>
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
                  className={homeSidebarStyles.sortTrigger}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent
                  position="popper"
                  side="bottom"
                  align="end"
                  className={homeSidebarStyles.sortContent}
                >
                  <SelectItem
                    value="lastPlayed"
                    className={homeSidebarStyles.sortItem}
                  >
                    <SortLabel icon={HistoryIcon} label="Last played" />
                  </SelectItem>
                  <SelectItem
                    value="name"
                    className={homeSidebarStyles.sortItem}
                  >
                    <SortLabel icon={ArrowDownAZIcon} label="A–Z" />
                  </SelectItem>
                  <SelectItem
                    value="nameDesc"
                    className={homeSidebarStyles.sortItem}
                  >
                    <SortLabel icon={ArrowDownZAIcon} label="Z–A" />
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            {loading ? (
              <p className={homeSidebarStyles.empty}>Loading…</p>
            ) : null}
            {!loading && ordered.length > 0 ? (
              <ScrollArea className={homeSidebarStyles.scrollArea}>
                <ul className={homeSidebarStyles.list}>
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
                        <li className={homeSidebarStyles.listItem}>
                          <Button
                            type="button"
                            variant={selected ? "secondary" : "ghost"}
                            className={cn(
                              homeSidebarStyles.instanceButton,
                              selected &&
                                homeSidebarStyles.instanceButtonSelected,
                              !selected && homeSidebarStyles.instanceButtonIdle,
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
                              className={homeSidebarStyles.instanceName}
                              title={instance.name}
                            >
                              {instance.name}
                            </span>
                            {active ? (
                              <span
                                aria-hidden
                                className={homeSidebarStyles.instanceStatus}
                              >
                                {phase === "starting" ? "Starting" : "Running"}
                                {phase === "starting" ? (
                                  <Loader2Icon
                                    className={
                                      homeSidebarStyles.instanceSpinner
                                    }
                                    strokeWidth={2}
                                  />
                                ) : (
                                  <span
                                    className={homeSidebarStyles.runningDot}
                                  >
                                    <span
                                      className={homeSidebarStyles.runningPing}
                                    />
                                    <span
                                      className={homeSidebarStyles.runningCore}
                                    />
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
