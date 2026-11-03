import { BoxIcon, LayersIcon, PuzzleIcon, TagIcon } from "lucide-react";

import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { PlatformPicker } from "@/features/new-instance/components/platform-picker";
import { SectionHeader } from "@/features/new-instance/components/section-header";
import { VersionList } from "@/features/new-instance/components/version-list";
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
    <section className="flex flex-col gap-3 border-t border-border/40 pt-4">
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

  return (
    <div className="grid gap-6 md:grid-cols-2">
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
        <section className="flex flex-col gap-3 border-t border-border/40 pt-4">
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
        {platformNeedsLoader(state.platform) ? (
          <LoaderOptions
            state={state}
            loaderVersions={loaderVersions}
            loaderVersionsPending={loaderVersionsPending}
            onChange={onChange}
          />
        ) : null}
      </div>
      <section className="flex min-h-0 flex-col gap-3">
        <SectionHeader
          icon={<LayersIcon aria-hidden className="size-4" strokeWidth={2} />}
          title="Game version"
          description="Missing versions install when you play."
        />
        <VersionList
          versions={versions}
          selected={state.gameVersion}
          pending={versionsPending}
          error={versionsError}
          includeSnapshots={state.includeSnapshots}
          onSelect={(gameVersion) => {
            onChange({ gameVersion, loaderVersion: "" });
          }}
          onIncludeSnapshotsChange={changeSnapshots}
          onRetry={onRetryVersions}
        />
      </section>
    </div>
  );
}
