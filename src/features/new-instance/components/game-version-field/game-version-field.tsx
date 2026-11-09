import { TagIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import { Switch } from "@/components/ui/switch";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import { FormSection } from "@/features/new-instance/components/form-section";
import type { ScratchState } from "@/features/new-instance/model/create-state";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

import { gameVersionFieldStyles } from "./game-version-field.styles";

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
      icon={TagIcon}
      title="Game version"
      description="Type to search. Missing versions install when you play."
    >
      {failed ? (
        <div className={gameVersionFieldStyles.root}>
          <p className={gameVersionFieldStyles.message}>
            Could not load the version list. Check your connection and try
            again.
          </p>
          <Button
            type="button"
            variant="outline"
            className={gameVersionFieldStyles.retryButton}
            onClick={onRetry}
          >
            Retry
          </Button>
        </div>
      ) : (
        <div className={gameVersionFieldStyles.row}>
          <Combobox
            aria-label="Game version"
            className={gameVersionFieldStyles.combobox}
            options={choices}
            value={state.gameVersion}
            disabled={pending}
            placeholder={pending ? "Loading versions…" : "Search versions"}
            emptyText="No versions match."
            onValueChange={(gameVersion) => {
              onChange({ gameVersion, loaderVersion: "" });
            }}
          />
          <div className={gameVersionFieldStyles.snapshotsToggle}>
            <Switch
              id="include-snapshots"
              aria-label="Include snapshots"
              checked={state.includeSnapshots}
              className={gameVersionFieldStyles.snapshotsSwitch}
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
