import { PuzzleIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { FormSection } from "@/features/new-instance/components/form-section";
import {
  type LoaderState,
  PLATFORMS,
  type ScratchState,
} from "@/features/new-instance/model/create-state";

export function LoaderOptions({
  state,
  loaderState,
  loaderVersions,
  onChange,
  onRetry,
}: {
  state: ScratchState;
  loaderState: LoaderState;
  loaderVersions: string[];
  onChange: (patch: Partial<ScratchState>) => void;
  onRetry: () => void;
}) {
  const loader =
    PLATFORMS.find((platform) => platform.id === state.platform)?.label ??
    state.platform;

  return (
    <FormSection
      icon={<PuzzleIcon aria-hidden className="size-4" strokeWidth={2} />}
      title="Loader version"
      description="Stable and latest are picked for the game version."
    >
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
      {loaderState === "checking" ? (
        <p className="text-sm text-muted-foreground">
          Checking {loader} builds…
        </p>
      ) : null}
      {loaderState === "empty" ? (
        <p className="text-sm text-destructive">
          {loader} has no build for {state.gameVersion}. Pick another version or
          platform.
        </p>
      ) : null}
      {loaderState === "error" ? (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-destructive">
            Could not check {loader} builds for {state.gameVersion}.
          </p>
          <Button
            type="button"
            variant="outline"
            className="self-start"
            onClick={onRetry}
          >
            Retry loader check
          </Button>
        </div>
      ) : null}
      {loaderState === "ready" && state.loaderChannel === "manual" ? (
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
      ) : null}
    </FormSection>
  );
}
