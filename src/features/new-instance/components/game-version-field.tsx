import { LayersIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import { Switch } from "@/components/ui/switch";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import { FormSection } from "@/features/new-instance/components/form-section";
import type { ScratchState } from "@/features/new-instance/model/create-state";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

export function GameVersionField({
  state,
  versions,
  pending,
  error,
  onChange,
  onRetry,
}: {
  state: ScratchState;
  versions: VersionSummary[];
  pending: boolean;
  error: boolean;
  onChange: (patch: Partial<ScratchState>) => void;
  onRetry: () => void;
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
  const failed = error && versions.length === 0;

  return (
    <FormSection
      icon={<LayersIcon aria-hidden className="size-4" strokeWidth={2} />}
      title="Game version"
      description="Type to search. Missing versions install when you play."
    >
      {failed ? (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-muted-foreground">
            Could not load the version list. Check your connection and try
            again.
          </p>
          <Button
            type="button"
            variant="outline"
            className="self-start"
            onClick={onRetry}
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
            disabled={pending}
            placeholder={pending ? "Loading versions…" : "Search versions"}
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
              className="data-[state=checked]:bg-warning"
              disabled={pending}
              onCheckedChange={changeSnapshots}
            />
            <label htmlFor="include-snapshots">Snapshots</label>
          </div>
        </div>
      )}
    </FormSection>
  );
}
