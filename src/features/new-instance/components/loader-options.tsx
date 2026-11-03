import { Reveal } from "@/components/ui/reveal";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ChannelPicker } from "@/features/new-instance/components/channel-picker";
import { FormSection } from "@/features/new-instance/components/form-section";
import {
  type LoaderState,
  PLATFORMS,
  type ScratchState,
} from "@/features/new-instance/model/create-state";

export function LoaderOptions({
  state,
  loaderState,
  stableMissing,
  loaderVersions,
  onChange,
}: {
  state: ScratchState;
  loaderState: LoaderState;
  /** The loader has no stable build for this game version. */
  stableMissing: boolean;
  loaderVersions: string[];
  onChange: (patch: Partial<ScratchState>) => void;
}) {
  const loader =
    PLATFORMS.find((platform) => platform.id === state.platform)?.label ??
    state.platform;

  return (
    <FormSection
      title="Loader version"
      description="Stable and Latest are picked for you. Other lets you choose a build."
    >
      <div>
        <ChannelPicker
          value={state.loaderChannel}
          loader={loader}
          gameVersion={state.gameVersion}
          stableMissing={stableMissing}
          noBuilds={loaderState === "empty"}
          failed={loaderState === "error"}
          onChange={(loaderChannel) => {
            onChange({ loaderChannel, loaderVersion: "" });
          }}
        />
        <Reveal
          open={loaderState === "ready" && state.loaderChannel === "manual"}
        >
          <div className="pt-2">
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
          </div>
        </Reveal>
      </div>
    </FormSection>
  );
}
