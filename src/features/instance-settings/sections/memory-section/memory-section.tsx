import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";
import { MemoryInput } from "@/features/settings/components/memory-input";
import { MemorySlider } from "@/features/settings/components/memory-slider";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

import { memorySectionStyles as styles } from "./memory-section.styles";

export function MemorySection({
  draft,
  snapshot,
  globalHint,
  onChange,
}: {
  draft: InstanceDraft;
  snapshot: SettingsSnapshot;
  /** What the instance uses while it does not override memory. */
  globalHint: string;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  return (
    <>
      <div className={styles.row}>
        <div className={styles.rowText}>
          <label htmlFor="override-memory" className={styles.rowLabel}>
            Customize memory allocation
          </label>
          <p id="override-memory-hint" className={styles.rowDescription}>
            {draft.overrideMemory
              ? "Set memory allocation separately for this instance. The minimum stays 512 MB."
              : globalHint}
          </p>
        </div>
        <Switch
          id="override-memory"
          className={styles.control}
          aria-label="Customize memory allocation"
          aria-describedby="override-memory-hint"
          checked={draft.overrideMemory}
          onCheckedChange={(overrideMemory) => onChange({ overrideMemory })}
        />
      </div>
      {draft.overrideMemory ? (
        <div className={styles.memoryField}>
          <div className={styles.row}>
            <span className={styles.rowLabel}>Memory</span>
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
        </div>
      ) : null}
    </>
  );
}
