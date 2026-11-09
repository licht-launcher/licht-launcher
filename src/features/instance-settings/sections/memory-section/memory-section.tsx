import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import { MemoryInput } from "@/features/settings/components/memory-input";
import { MemorySlider } from "@/features/settings/components/memory-slider";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

import { memorySectionStyles } from "./memory-section.styles";

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
    <div className={memorySectionStyles.root}>
      <div className={memorySectionStyles.toggle}>
        <div className={memorySectionStyles.toggleRow}>
          <label
            htmlFor="override-memory"
            className={memorySectionStyles.label}
          >
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
        <p id="override-memory-hint" className={memorySectionStyles.hint}>
          Set memory allocation separately for this instance.
        </p>
      </div>
      {draft.overrideMemory ? (
        <div className={memorySectionStyles.fields}>
          <div className={memorySectionStyles.valueRow}>
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
          <p className={memorySectionStyles.hint}>
            Memory allocation. The minimum stays 512 MB.
          </p>
        </div>
      ) : null}
    </div>
  );
}
