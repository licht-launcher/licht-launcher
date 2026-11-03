import { PuzzleIcon } from "lucide-react";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { FormSection } from "@/features/new-instance/components/form-section";
import type { ScratchState } from "@/features/new-instance/model/create-state";

export function LoaderOptions({
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
    </FormSection>
  );
}
