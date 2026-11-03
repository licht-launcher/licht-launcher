import { BoxIcon, LayersIcon, PuzzleIcon, TagIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import { PlatformPicker } from "@/features/new-instance/components/platform-picker";
import { SectionHeader } from "@/features/new-instance/components/section-header";
import {
  platformNeedsLoader,
  type ScratchState,
} from "@/features/new-instance/model/create-state";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

function LoaderOptions({
  state,
  loaderVersions,
  loaderVersionsPending,
  onChange,
}: {
  state: ScratchState;
  loaderVersions: string[];
  loaderVersionsPending: boolean;
  onChange: (patch: Partial<ScratchState>) => void;
}) {
  return (
    <section className="flex flex-col gap-3 border-t border-border/40 pt-5">
      <SectionHeader
        icon={<PuzzleIcon aria-hidden className="size-4" strokeWidth={2} />}
        title="Loader version"
        description="Stable and latest are picked for the game version."
      />
      <Select
        value={state.loaderChannel}
        onValueChange={(value) => {
          if (value === "stable" || value === "latest" || value === "manual") {
            onChange({ loaderChannel: value, loaderVersion: "" });
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
      {state.loaderChannel === "manual" ? (
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
            value={state.loaderVersion}
            onValueChange={(loaderVersion) => {
              onChange({ loaderVersion });
            }}
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
    </section>
  );
}

export function ScratchDetails({
  state,
  versions,
  versionsPending,
  versionsError,
  loaderVersions,
  loaderVersionsPending,
  onRetryVersions,
  onChange,
}: {
  state: ScratchState;
  versions: VersionSummary[];
  versionsPending: boolean;
  versionsError: boolean;
  loaderVersions: string[];
  loaderVersionsPending: boolean;
  onRetryVersions: () => void;
  onChange: (patch: Partial<ScratchState>) => void;
}) {
  function changeSnapshots(includeSnapshots: boolean) {
    const selectedIsSnapshot =
      versions.find((version) => version.id === state.gameVersion)
        ?.versionType === "snapshot";
    // A hidden snapshot must not stay selected; the default picks the newest release.
    onChange({
      includeSnapshots,
      ...(!includeSnapshots && selectedIsSnapshot
        ? { gameVersion: "", loaderVersion: "" }
        : {}),
    });
  }

  const choices = createVersionChoices(versions, state.includeSnapshots).map(
    (version) => ({ value: version.id, label: version.id }),
  );
  const versionsFailed = versionsError && versions.length === 0;

  return (
    <div className="flex flex-col gap-5">
      <section className="flex flex-col gap-3">
        <SectionHeader
          icon={<TagIcon aria-hidden className="size-4" strokeWidth={2} />}
          title="Name your instance"
          description="You can rename it later from the library."
        />
        <label
          className="flex flex-col gap-1.5 text-sm font-medium"
          htmlFor="instance-name"
        >
          Instance Name
          <Input
            id="instance-name"
            value={state.name}
            autoComplete="off"
            placeholder="Survival"
            onChange={(event) => {
              onChange({ name: event.target.value });
            }}
          />
        </label>
      </section>
      <section className="flex flex-col gap-3 border-t border-border/40 pt-5">
        <SectionHeader
          icon={<BoxIcon aria-hidden className="size-4" strokeWidth={2} />}
          title="Choose a platform"
          description="Vanilla is the official game. Loaders add mod support."
        />
        <PlatformPicker
          value={state.platform}
          onChange={(platform) => {
            onChange({ platform, loaderVersion: "" });
          }}
        />
      </section>
      <section className="flex flex-col gap-3 border-t border-border/40 pt-5">
        <SectionHeader
          icon={<LayersIcon aria-hidden className="size-4" strokeWidth={2} />}
          title="Game version"
          description="Type to search. Missing versions install when you play."
        />
        {versionsFailed ? (
          <div className="flex flex-col gap-2">
            <p className="text-sm text-muted-foreground">
              Could not load the version list. Check your connection and try
              again.
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
        ) : (
          <div className="flex items-center gap-3">
            <Combobox
              aria-label="Game version"
              className="flex-1"
              options={choices}
              value={state.gameVersion}
              disabled={versionsPending}
              placeholder={
                versionsPending ? "Loading versions…" : "Search versions"
              }
              emptyText="No versions match."
              onValueChange={(gameVersion) => {
                onChange({ gameVersion, loaderVersion: "" });
              }}
            />
            <div className="flex shrink-0 items-center gap-2 text-sm text-muted-foreground">
              <Switch
                id="include-snapshots"
                aria-label="Include snapshots"
                checked={state.includeSnapshots}
                disabled={versionsPending}
                onCheckedChange={changeSnapshots}
              />
              <label htmlFor="include-snapshots">Snapshots</label>
            </div>
          </div>
        )}
      </section>
      {platformNeedsLoader(state.platform) ? (
        <LoaderOptions
          state={state}
          loaderVersions={loaderVersions}
          loaderVersionsPending={loaderVersionsPending}
          onChange={onChange}
        />
      ) : null}
    </div>
  );
}
