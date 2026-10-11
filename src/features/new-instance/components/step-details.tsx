import { CalendarRangeIcon, PuzzleIcon, TagIcon } from "lucide-react";

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
import {
  type LoaderChannel,
  type PlatformId,
  platformNeedsLoader,
} from "@/features/new-instance/model/wizard";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

export function StepDetails({
  platform,
  name,
  gameVersion,
  includeSnapshots,
  versions,
  versionsPending,
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
  const showLoader = platformNeedsLoader(platform);

  return (
    <div className="flex max-w-md flex-col gap-4">
      <div className="flex items-center gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-success/25 text-success">
          <TagIcon aria-hidden className="size-5" strokeWidth={2} />
        </span>
        <div>
          <h2 className="text-lg font-medium">
            {showLoader ? "Details and loader" : "Name and game version"}
          </h2>
          <p className="text-sm text-muted-foreground">
            Missing versions install when you play.
          </p>
        </div>
      </div>
      <label className="flex flex-col gap-1 text-sm" htmlFor="instance-name">
        Name
        <Input
          id="instance-name"
          value={name}
          autoComplete="off"
          onChange={(event) => {
            onNameChange(event.target.value);
          }}
        />
      </label>
      {platform === "vanilla" ? (
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 items-start gap-2">
            <CalendarRangeIcon
              aria-hidden
              className="mt-0.5 size-4 shrink-0 text-muted-foreground"
            />
            <div className="flex min-w-0 flex-col gap-1">
              <label
                htmlFor="include-snapshots"
                className="text-sm font-medium"
              >
                Include snapshots
              </label>
              <p
                id="include-snapshots-hint"
                className="text-sm text-muted-foreground"
              >
                Show snapshot builds in the version list.
              </p>
            </div>
          </div>
          <Switch
            id="include-snapshots"
            className="shrink-0"
            aria-label="Include snapshots"
            aria-describedby="include-snapshots-hint"
            checked={includeSnapshots}
            onCheckedChange={(next) => {
              onIncludeSnapshotsChange(next);
              const nextChoices = createVersionChoices(versions, next);
              if (!nextChoices.some((version) => version.id === gameVersion)) {
                onGameVersionChange(nextChoices[0]?.id ?? "");
              }
            }}
          />
        </div>
      ) : null}
      {choices.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          {versionsPending ? "Loading versions…" : "No versions available."}
        </p>
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
      )}
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
