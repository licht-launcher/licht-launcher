import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import { MemoryInput } from "@/features/settings/components/memory-input";
import { MemorySlider } from "@/features/settings/components/memory-slider";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

export function MemorySection({
  draft,
  snapshot,
  onChange,
}: {
  draft: InstanceDraft;
  snapshot: SettingsSnapshot;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-3">
          <label htmlFor="override-memory" className="text-sm">
            Customize memory allocation
          </label>
          <Switch
            id="override-memory"
            aria-label="Customize memory allocation"
            aria-describedby="override-memory-hint"
            checked={draft.overrideMemory}
            onCheckedChange={(overrideMemory) => onChange({ overrideMemory })}
          />
        </div>
        <p id="override-memory-hint" className="text-sm text-muted-foreground">
          Set memory allocation separately for this instance.
        </p>
      </div>
      {draft.overrideMemory ? (
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between gap-3 text-sm">
            <span>Memory</span>
            <MemoryInput
              valueMb={draft.maxMemoryMb}
              minimumMb={snapshot.memoryMinimumMb}
              maximumMb={snapshot.memoryMaximumMb}
              onChange={(maxMemoryMb) => onChange({ maxMemoryMb })}
            />
          </div>
          <MemorySlider
            valueMb={draft.maxMemoryMb}
            minimumMb={snapshot.memoryMinimumMb}
            maximumMb={snapshot.memoryMaximumMb}
            recommendedMb={snapshot.memoryRecommendedMb}
            totalMb={snapshot.memoryTotalMb}
            stepMb={snapshot.memoryStepMb}
            onChange={(maxMemoryMb) => {
              onChange({ maxMemoryMb });
            }}
          />
          <p className="text-sm text-muted-foreground">
            Memory allocation. The minimum stays 512 MB.
          </p>
        </div>
      ) : null}
    </div>
  );
}
