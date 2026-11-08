import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { MemoryInput } from "@/features/settings/components/memory-input";
import { MemorySlider } from "@/features/settings/components/memory-slider";
import { useMemoryPreview } from "@/features/settings/hooks/use-memory-preview";
import type { Draft } from "@/features/settings/model/draft";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

import { instancesSectionStyles } from "./instances-section.styles";

export function InstancesSection({
  draft,
  snapshot,
  onChange,
}: {
  draft: Draft;
  snapshot: SettingsSnapshot;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const memoryPreview = useMemoryPreview();
  const previewing = memoryPreview.totalMb !== null;
  const maximumMb = memoryPreview.totalMb ?? snapshot.memoryMaximumMb;
  const allocationMb = previewing
    ? Math.max(
        snapshot.memoryMinimumMb,
        Math.min(maximumMb, memoryPreview.allocationMb ?? draft.maxMemoryMb),
      )
    : draft.maxMemoryMb;
  function updateMemory(maxMemoryMb: number) {
    if (previewing)
      memoryPreview.setAllocationMb(maxMemoryMb, snapshot.memoryMinimumMb);
    else onChange({ maxMemoryMb });
  }
  return (
    <div className={instancesSectionStyles.root}>
      <Card title="Game window" aria-label="Game window">
        <div className={instancesSectionStyles.row}>
          <div className={instancesSectionStyles.rowText}>
            <label
              htmlFor="fullscreen"
              className={instancesSectionStyles.rowLabel}
            >
              Fullscreen
            </label>
            <p
              id="fullscreen-hint"
              className={instancesSectionStyles.rowDescription}
            >
              The game starts in fullscreen. Width and height are not used.
            </p>
          </div>
          <Switch
            id="fullscreen"
            className={instancesSectionStyles.control}
            aria-label="Fullscreen"
            aria-describedby="fullscreen-hint"
            checked={draft.fullscreen}
            onCheckedChange={(fullscreen) => onChange({ fullscreen })}
          />
        </div>
        <div className={instancesSectionStyles.row}>
          <div className={instancesSectionStyles.rowText}>
            <label htmlFor="width" className={instancesSectionStyles.rowLabel}>
              Window width
            </label>
            <p
              id="width-hint"
              className={instancesSectionStyles.rowDescription}
            >
              Window width in pixels.
            </p>
          </div>
          <Input
            id="width"
            type="number"
            min={1}
            inputMode="numeric"
            placeholder="854"
            className={instancesSectionStyles.numberInput}
            aria-describedby="width-hint"
            value={draft.width}
            disabled={draft.fullscreen}
            onChange={(event) => onChange({ width: event.target.value })}
          />
        </div>
        <div className={instancesSectionStyles.row}>
          <div className={instancesSectionStyles.rowText}>
            <label htmlFor="height" className={instancesSectionStyles.rowLabel}>
              Window height
            </label>
            <p
              id="height-hint"
              className={instancesSectionStyles.rowDescription}
            >
              Window height in pixels.
            </p>
          </div>
          <Input
            id="height"
            type="number"
            min={1}
            inputMode="numeric"
            placeholder="480"
            className={instancesSectionStyles.numberInput}
            aria-describedby="height-hint"
            value={draft.height}
            disabled={draft.fullscreen}
            onChange={(event) => onChange({ height: event.target.value })}
          />
        </div>
      </Card>

      <Card title="Memory" aria-label="Memory">
        <div className={instancesSectionStyles.memoryField}>
          <div className={instancesSectionStyles.row}>
            <span className={instancesSectionStyles.rowLabel}>
              Memory Allocation
            </span>
            <MemoryInput
              valueMb={allocationMb}
              minimumMb={snapshot.memoryMinimumMb}
              maximumMb={maximumMb}
              onChange={updateMemory}
            />
          </div>
          <p id="memory-hint" className={instancesSectionStyles.rowDescription}>
            Memory allocation. The minimum stays 512 MB.
          </p>
          {memoryPreview.totalMb !== null ? (
            <p className={instancesSectionStyles.rowDescription}>
              Preview: {memoryPreview.totalMb / 1024} GiB RAM. Allocation edits
              are kept in this session.
            </p>
          ) : null}
        </div>
        <MemorySlider
          valueMb={allocationMb}
          minimumMb={snapshot.memoryMinimumMb}
          maximumMb={maximumMb}
          recommendedMb={snapshot.memoryRecommendedMb}
          totalMb={memoryPreview.totalMb ?? snapshot.memoryTotalMb}
          stepMb={snapshot.memoryStepMb}
          onChange={updateMemory}
        />
      </Card>
    </div>
  );
}
