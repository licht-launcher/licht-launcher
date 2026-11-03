import { BoxIcon } from "lucide-react";

import { Reveal } from "@/components/ui/reveal";
import { FormSection } from "@/features/new-instance/components/form-section";
import { GameVersionField } from "@/features/new-instance/components/game-version-field";
import { LoaderOptions } from "@/features/new-instance/components/loader-options";
import { NameField } from "@/features/new-instance/components/name-field";
import { PlatformPicker } from "@/features/new-instance/components/platform-picker";
import {
  defaultInstanceName,
  isNameTaken,
  type LoaderState,
  platformNeedsLoader,
  type ScratchState,
} from "@/features/new-instance/model/create-state";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

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
    <div className="flex flex-col">
      <div className="flex flex-col gap-5">
        <NameField
          value={state.name}
          placeholder={
            defaultInstanceName(state, takenNames) || "Instance name"
          }
          error={
            isNameTaken(state.name, takenNames)
              ? "An instance with this name already exists."
              : undefined
          }
          onChange={(name) => {
            onChange({ name });
          }}
        />
        <FormSection
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
        <div className="pt-4">
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
