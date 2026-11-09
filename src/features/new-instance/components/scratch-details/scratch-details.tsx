import { BoxIcon } from "lucide-react";

import { Reveal } from "@/components/ui/reveal";
import { FormSection } from "@/features/new-instance/components/form-section";
import { GameVersionField } from "@/features/new-instance/components/game-version-field";
import { LoaderOptions } from "@/features/new-instance/components/loader-options";
import { NameField } from "@/features/new-instance/components/name-field";
import { PlatformPicker } from "@/features/new-instance/components/platform-picker";
import {
  defaultInstanceName,
  type LoaderState,
  platformNeedsLoader,
  type ScratchState,
} from "@/features/new-instance/model/create-state";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

import { scratchDetailsStyles } from "./scratch-details.styles";

export function ScratchDetails({
  state,
  takenNames,
  loaderState,
  stableMissing,
  versions,
  versionsPending,
  versionsError,
  loaderVersions,
  onRetryVersions,
  onChange,
}: {
  state: ScratchState;
  /** Names already used by other instances. */
  takenNames: readonly string[];
  loaderState: LoaderState;
  stableMissing: boolean;
  versions: VersionSummary[];
  versionsPending: boolean;
  versionsError: boolean;
  loaderVersions: string[];
  onRetryVersions: () => void;
  onChange: (patch: Partial<ScratchState>) => void;
}) {
  return (
    <div className={scratchDetailsStyles.root}>
      <div className={scratchDetailsStyles.sections}>
        <NameField
          value={state.name}
          placeholder={
            defaultInstanceName(state, takenNames) || "Instance name"
          }
          onChange={(name) => {
            onChange({ name });
          }}
        />
        <FormSection
          icon={BoxIcon}
          title="Choose a platform"
          description="Vanilla is the official game. Loaders add mod support."
        >
          <PlatformPicker
            value={state.platform}
            onChange={(platform) => {
              onChange({
                platform,
                loaderChannel: "stable",
                loaderVersion: "",
              });
            }}
          />
        </FormSection>
        <GameVersionField
          state={state}
          versions={versions}
          pending={versionsPending}
          error={versionsError}
          onChange={onChange}
          onRetry={onRetryVersions}
        />
      </div>
      <Reveal open={platformNeedsLoader(state.platform)}>
        <div className={scratchDetailsStyles.loaderOptions}>
          <LoaderOptions
            state={state}
            loaderState={loaderState}
            stableMissing={stableMissing}
            loaderVersions={loaderVersions}
            onChange={onChange}
          />
        </div>
      </Reveal>
    </div>
  );
}
