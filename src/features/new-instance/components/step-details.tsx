import {
  BoxIcon,
  CheckIcon,
  ChevronDownIcon,
  CuboidIcon,
  FlaskConicalIcon,
  FoldVerticalIcon,
  HistoryIcon,
  LayersIcon,
  PuzzleIcon,
  SearchIcon,
  UnfoldVerticalIcon,
} from "lucide-react";
import {
  type ReactNode,
  type RefObject,
  useEffect,
  useRef,
  useState,
} from "react";

import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  createVersionChoices,
  filterVersionGroups,
  groupVersionsByMajor,
  updateNameForMajor,
  versionMajorGroup,
} from "@/features/instances/model/create-versions";
import {
  type LoaderChannel,
  type PlatformId,
  platformNeedsLoader,
} from "@/features/new-instance/model/wizard";
import type { VersionSummary } from "@/lib/generated/VersionSummary";
import type { VersionType } from "@/lib/generated/VersionType";
import { cn } from "@/lib/utils";

const MAJOR_TONES = [
  "bg-instance/20 text-instance",
  "bg-accent-cool/20 text-accent-cool",
  "bg-warning/20 text-warning",
  "bg-accent-file/20 text-accent-file",
  "bg-accent-rose/20 text-accent-rose",
  "bg-accent-orange/20 text-accent-orange",
  "bg-accent-violet/20 text-accent-violet",
  "bg-accent-teal/20 text-accent-teal",
  "bg-info/20 text-info",
  "bg-success/20 text-success",
] as const;

/** Same tone for 5 majors, then the next color. */
function majorToneClass(groupIndex: number): string {
  return (
    MAJOR_TONES[Math.floor(groupIndex / 5) % MAJOR_TONES.length] ??
    MAJOR_TONES[0]
  );
}

function majorBadgeLabel(major: string): string {
  if (major === "Snapshots") {
    return "S";
  }
  const parts = major.split(".");
  return parts[1] ?? major.slice(0, 2);
}

function IconField({
  icon,
  label,
  htmlFor,
  children,
  className,
}: {
  icon: ReactNode;
  label: ReactNode;
  htmlFor?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex min-h-0 gap-2", className)}>
      <span className="mt-0.5 flex size-4 shrink-0 items-center justify-center text-muted-foreground">
        {icon}
      </span>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-1.5">
        {htmlFor !== undefined ? (
          <label
            htmlFor={htmlFor}
            className="text-sm font-semibold text-foreground"
          >
            {label}
          </label>
        ) : (
          <p className="text-sm font-semibold text-foreground">{label}</p>
        )}
        {children}
      </div>
    </div>
  );
}

const SNAPSHOT_TONE = "bg-warning/20 text-warning";

function VersionRowIcon({
  versionType,
  toneClass,
}: {
  versionType: VersionType;
  toneClass: string;
}) {
  const Icon =
    versionType === "snapshot"
      ? FlaskConicalIcon
      : versionType === "old_beta" || versionType === "old_alpha"
        ? HistoryIcon
        : CuboidIcon;
  return (
    <span
      aria-hidden
      className={cn(
        "flex size-5 shrink-0 items-center justify-center rounded-md",
        versionType === "snapshot" ? SNAPSHOT_TONE : toneClass,
      )}
    >
      <Icon className="size-3" strokeWidth={2.25} />
    </span>
  );
}

function LatestTag() {
  return (
    <span className="inline-flex shrink-0 items-center rounded-full border border-primary-soft/60 bg-primary-soft/20 px-2 py-0.5 text-xs font-semibold text-primary-soft">
      Latest
    </span>
  );
}

function useScrollFog(
  rootRef: RefObject<HTMLElement | null>,
  contentKey: string,
) {
  const [canScrollUp, setCanScrollUp] = useState(false);
  const [canScrollDown, setCanScrollDown] = useState(false);

  useEffect(() => {
    const root = rootRef.current;
    if (root === null) {
      return;
    }
    const viewport = root.querySelector("[data-slot='scroll-area-viewport']");
    if (!(viewport instanceof HTMLElement)) {
      return;
    }

    const update = () => {
      const { scrollTop, scrollHeight, clientHeight } = viewport;
      setCanScrollUp(scrollTop > 1);
      setCanScrollDown(scrollTop + clientHeight < scrollHeight - 1);
    };

    // Remeasure when filter / open groups change.
    void contentKey;
    update();
    viewport.addEventListener("scroll", update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(viewport);
    const content = viewport.firstElementChild;
    if (content instanceof HTMLElement) {
      observer.observe(content);
    }

    return () => {
      viewport.removeEventListener("scroll", update);
      observer.disconnect();
    };
  }, [rootRef, contentKey]);

  return { canScrollUp, canScrollDown };
}

export function StepDetails({
  platform,
  name,
  gameVersion,
  includeSnapshots,
  versions,
  versionsPending,
  versionsError,
  onRetryVersions,
  loaderChannel,
  loaderVersion,
  loaderVersions,
  loaderVersionsPending,
  onNameChange,
  onGameVersionChange,
  onIncludeSnapshotsChange,
  onLoaderChannelChange,
  onLoaderVersionChange,
}: {
  platform: PlatformId | null;
  name: string;
  gameVersion: string;
  includeSnapshots: boolean;
  versions: VersionSummary[];
  versionsPending: boolean;
  versionsError: boolean;
  onRetryVersions: () => void;
  loaderChannel: LoaderChannel;
  loaderVersion: string;
  loaderVersions: string[];
  loaderVersionsPending: boolean;
  onNameChange: (name: string) => void;
  onGameVersionChange: (versionId: string) => void;
  onIncludeSnapshotsChange: (include: boolean) => void;
  onLoaderChannelChange: (channel: LoaderChannel) => void;
  onLoaderVersionChange: (version: string) => void;
}) {
  const choices = createVersionChoices(versions, includeSnapshots);
  const groups = groupVersionsByMajor(choices);
  const newestVersionId = choices[0]?.id ?? "";
  const newestMajor = groups[0]?.major ?? "";
  const showLoader = platformNeedsLoader(platform);
  const vanilla = platform === "vanilla";
  const selectedMajor =
    gameVersion === ""
      ? (groups[0]?.major ?? "")
      : versionMajorGroup(gameVersion);

  const [versionQuery, setVersionQuery] = useState("");
  const [openMajors, setOpenMajors] = useState<Set<string>>(
    () => new Set(selectedMajor === "" ? [] : [selectedMajor]),
  );
  const versionScrollRef = useRef<HTMLDivElement>(null);
  const hasSnapshotsGroup = groups.some((group) => group.major === "Snapshots");
  const visibleGroups = filterVersionGroups(groups, versionQuery);
  const searching = versionQuery.trim() !== "";
  const allVisibleOpen =
    visibleGroups.length > 0 &&
    (searching || visibleGroups.every((group) => openMajors.has(group.major)));
  const scrollFogKey = `${versionQuery}:${includeSnapshots}:${visibleGroups
    .map((group) => `${group.major}:${openMajors.has(group.major) ? "1" : "0"}`)
    .join(",")}`;
  const { canScrollUp, canScrollDown } = useScrollFog(
    versionScrollRef,
    scrollFogKey,
  );

  useEffect(() => {
    setOpenMajors((current) => {
      let changed = false;
      const next = new Set(current);
      if (selectedMajor !== "" && !next.has(selectedMajor)) {
        next.add(selectedMajor);
        changed = true;
      }
      if (includeSnapshots && hasSnapshotsGroup && !next.has("Snapshots")) {
        next.add("Snapshots");
        changed = true;
      }
      return changed ? next : current;
    });
  }, [selectedMajor, includeSnapshots, hasSnapshotsGroup]);

  function toggleSnapshots() {
    const next = !includeSnapshots;
    onIncludeSnapshotsChange(next);
    const nextChoices = createVersionChoices(versions, next);
    if (!nextChoices.some((version) => version.id === gameVersion)) {
      onGameVersionChange(nextChoices[0]?.id ?? "");
    }
  }

  function toggleAllMajors() {
    if (allVisibleOpen) {
      setOpenMajors(new Set());
      if (searching) {
        setVersionQuery("");
      }
      return;
    }
    setOpenMajors(new Set(visibleGroups.map((group) => group.major)));
  }

  const nameField = (
    <IconField
      htmlFor="instance-name"
      label="Instance Name"
      icon={<BoxIcon aria-hidden className="size-4" strokeWidth={2} />}
    >
      <Input
        id="instance-name"
        value={name}
        autoComplete="off"
        placeholder="Survival"
        onChange={(event) => {
          onNameChange(event.target.value);
        }}
      />
    </IconField>
  );

  const versionToolbar = (
    <div className="flex flex-wrap items-center gap-2 pr-2">
      <div className="relative min-w-48 flex-1">
        <SearchIcon
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
          strokeWidth={2}
        />
        <Input
          id="version-search"
          value={versionQuery}
          autoComplete="off"
          placeholder="Search versions"
          aria-label="Search versions"
          className="pl-8"
          onChange={(event) => {
            setVersionQuery(event.target.value);
          }}
        />
      </div>
      <Button
        type="button"
        size="sm"
        variant={includeSnapshots ? "warning" : "outline"}
        aria-pressed={includeSnapshots}
        aria-label="Include snapshots"
        className={cn(
          !includeSnapshots &&
            "border-warning/50 text-warning hover:bg-warning/10 hover:text-warning",
        )}
        onClick={toggleSnapshots}
      >
        <FlaskConicalIcon
          aria-hidden
          data-icon="inline-start"
          strokeWidth={2}
        />
        Snapshots
      </Button>
      <Button
        type="button"
        size="sm"
        variant="outline"
        aria-label={allVisibleOpen ? "Collapse all" : "Expand all"}
        disabled={visibleGroups.length === 0}
        onClick={toggleAllMajors}
      >
        {allVisibleOpen ? (
          <FoldVerticalIcon
            aria-hidden
            data-icon="inline-start"
            strokeWidth={2}
          />
        ) : (
          <UnfoldVerticalIcon
            aria-hidden
            data-icon="inline-start"
            strokeWidth={2}
          />
        )}
        {allVisibleOpen ? "Collapse all" : "Expand all"}
      </Button>
    </div>
  );

  const vanillaVersionList = versionsPending ? (
    <p className="text-sm text-muted-foreground">Loading versions…</p>
  ) : versionsError && choices.length === 0 ? (
    <div className="flex flex-col gap-2">
      <p className="text-sm text-muted-foreground">
        Could not load the version list. Check your connection and try again.
      </p>
      <Button
        type="button"
        variant="outline"
        className="self-start"
        onClick={onRetryVersions}
      >
        Retry
      </Button>
    </div>
  ) : choices.length === 0 ? (
    <p className="text-sm text-muted-foreground">No versions available.</p>
  ) : visibleGroups.length === 0 ? (
    <p className="text-sm text-muted-foreground">No versions match.</p>
  ) : (
    <div ref={versionScrollRef} className="relative min-h-0 w-full flex-1">
      <ScrollArea className="h-full min-h-0 pr-2">
        <div className="overflow-hidden rounded-lg border border-border/60 bg-black/20">
          {visibleGroups.map((group) => {
            const groupIndex = groups.findIndex(
              (entry) => entry.major === group.major,
            );
            const open = searching || openMajors.has(group.major);
            const updateName = updateNameForMajor(group.major);
            const isLatestMajor = group.major === newestMajor;
            const toneClass =
              group.major === "Snapshots"
                ? SNAPSHOT_TONE
                : majorToneClass(groupIndex < 0 ? 0 : groupIndex);
            return (
              <Collapsible
                key={group.major}
                open={open}
                className="border-b border-border/60 last:border-b-0"
                onOpenChange={(next) => {
                  setOpenMajors((current) => {
                    const updated = new Set(current);
                    if (next) {
                      updated.add(group.major);
                    } else {
                      updated.delete(group.major);
                    }
                    return updated;
                  });
                }}
              >
                <CollapsibleTrigger
                  type="button"
                  aria-label={`${group.major} versions`}
                  className={cn(
                    "flex w-full cursor-pointer items-center gap-2 px-3.5 py-2.5 text-left text-sm font-medium outline-none",
                    "hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring/50",
                  )}
                >
                  <ChevronDownIcon
                    aria-hidden
                    className={cn(
                      "size-4 shrink-0 text-muted-foreground transition-transform duration-200",
                      open && "rotate-180",
                    )}
                    strokeWidth={2}
                  />
                  <span
                    aria-hidden
                    className={cn(
                      "flex size-6 shrink-0 items-center justify-center rounded-md text-[0.65rem] font-semibold",
                      toneClass,
                    )}
                  >
                    {majorBadgeLabel(group.major)}
                  </span>
                  <span className="flex min-w-0 flex-1 flex-col items-start gap-0.5">
                    <span className="flex w-full min-w-0 items-center gap-2">
                      <span className="truncate">{group.major}</span>
                      {isLatestMajor ? <LatestTag /> : null}
                    </span>
                    {updateName !== null ? (
                      <span className="w-full truncate text-xs font-normal text-muted-foreground">
                        {updateName}
                      </span>
                    ) : null}
                  </span>
                  <span className="shrink-0 text-xs font-normal text-muted-foreground">
                    {group.versions.length}
                  </span>
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <ul className="flex flex-col border-t border-border/60 pb-1">
                    {group.versions.map((version) => {
                      const selected = version.id === gameVersion;
                      const isLatest = version.id === newestVersionId;
                      return (
                        <li key={version.id}>
                          <button
                            type="button"
                            aria-pressed={selected}
                            aria-label={version.id}
                            className={cn(
                              "flex w-full cursor-pointer items-center gap-3 px-3.5 py-2.5 pl-10 text-left outline-none transition-colors duration-200 ease-out",
                              "hover:bg-muted/25 focus-visible:bg-muted/25 focus-visible:ring-2 focus-visible:ring-ring/50",
                              selected && "bg-muted/35",
                            )}
                            onClick={() => {
                              onGameVersionChange(version.id);
                            }}
                          >
                            {selected ? (
                              <span
                                aria-hidden
                                className="flex size-5 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground"
                              >
                                <CheckIcon className="size-3" strokeWidth={3} />
                              </span>
                            ) : (
                              <VersionRowIcon
                                versionType={version.versionType}
                                toneClass={toneClass}
                              />
                            )}
                            <span className="min-w-0 flex-1 truncate text-sm font-medium">
                              {version.id}
                            </span>
                            {isLatest ? <LatestTag /> : null}
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                </CollapsibleContent>
              </Collapsible>
            );
          })}
        </div>
      </ScrollArea>
      <div
        aria-hidden
        className={cn(
          "pointer-events-none absolute inset-x-0 top-0 z-10 h-16 w-full",
          "transition-opacity duration-200 ease-out",
          canScrollUp ? "opacity-100" : "opacity-0",
        )}
      >
        {/* Same stack as the main panel: popover + muted/30 */}
        <div className="absolute inset-0 bg-gradient-to-b from-popover via-popover/55 to-transparent" />
        <div className="absolute inset-0 bg-gradient-to-b from-muted/30 via-muted/15 to-transparent" />
      </div>
      <div
        aria-hidden
        className={cn(
          "pointer-events-none absolute inset-x-0 bottom-0 z-10 h-16 w-full",
          "transition-opacity duration-200 ease-out",
          canScrollDown ? "opacity-100" : "opacity-0",
        )}
      >
        <div className="absolute inset-0 bg-gradient-to-t from-popover via-popover/55 to-transparent" />
        <div className="absolute inset-0 bg-gradient-to-t from-muted/30 via-muted/15 to-transparent" />
      </div>
    </div>
  );

  const loaderVersionSelect = versionsPending ? (
    <p className="text-sm text-muted-foreground">Loading versions…</p>
  ) : versionsError && choices.length === 0 ? (
    <div className="flex flex-col gap-2">
      <p className="text-sm text-muted-foreground">
        Could not load the version list. Check your connection and try again.
      </p>
      <Button
        type="button"
        variant="outline"
        className="self-start"
        onClick={onRetryVersions}
      >
        Retry
      </Button>
    </div>
  ) : choices.length === 0 ? (
    <p className="text-sm text-muted-foreground">No versions available.</p>
  ) : (
    <Select value={gameVersion} onValueChange={onGameVersionChange}>
      <SelectTrigger aria-label="Game version">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {choices.map((version) => (
          <SelectItem key={version.id} value={version.id}>
            {version.id}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );

  const header = (
    <div>
      <h2 className="text-lg font-medium">
        {showLoader ? "Details and loader" : "Name and game version"}
      </h2>
      <p className="text-sm text-muted-foreground">
        Missing versions install when you play.
      </p>
    </div>
  );

  if (vanilla) {
    return (
      <div className="flex min-h-0 w-full max-w-4xl flex-1 flex-col gap-4">
        {header}
        <div className="max-w-md">{nameField}</div>
        <IconField
          className="min-h-0 flex-1"
          label="Game version"
          icon={<LayersIcon aria-hidden className="size-4" strokeWidth={2} />}
        >
          {versionToolbar}
          <div className="flex min-h-0 flex-1 flex-col">
            {vanillaVersionList}
          </div>
        </IconField>
      </div>
    );
  }

  return (
    <div className="flex max-w-md flex-col gap-4">
      {header}
      {nameField}
      {loaderVersionSelect}
      {showLoader ? (
        <div className="flex flex-col gap-3 border-t border-border/40 pt-4">
          <div className="flex items-center gap-2 text-sm font-medium">
            <PuzzleIcon
              aria-hidden
              className="size-4 shrink-0 text-muted-foreground"
            />
            Loader version
          </div>
          <Select
            value={loaderChannel}
            onValueChange={(value) => {
              if (
                value === "stable" ||
                value === "latest" ||
                value === "manual"
              ) {
                onLoaderChannelChange(value);
              }
            }}
          >
            <SelectTrigger aria-label="Loader channel">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="stable">Stable</SelectItem>
              <SelectItem value="latest">Latest</SelectItem>
              <SelectItem value="manual">Select manually</SelectItem>
            </SelectContent>
          </Select>
          {loaderChannel === "manual" ? (
            loaderVersionsPending ? (
              <p className="text-sm text-muted-foreground">
                Loading loader versions…
              </p>
            ) : loaderVersions.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No loader versions for this game version.
              </p>
            ) : (
              <Select
                value={loaderVersion}
                onValueChange={onLoaderVersionChange}
              >
                <SelectTrigger aria-label="Loader version">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {loaderVersions.map((version) => (
                    <SelectItem key={version} value={version}>
                      {version}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
