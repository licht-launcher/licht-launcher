import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { MemoryInput } from "@/features/settings/components/memory-input";
import { MemorySlider } from "@/features/settings/components/memory-slider";
import { useMemoryPreview } from "@/features/settings/hooks/use-memory-preview";
import type { Draft } from "@/features/settings/model/draft";
import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";

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
    <div className="flex flex-col gap-8">
      <Card title="Game window" aria-label="Game window">
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            <label htmlFor="fullscreen" className="text-sm font-normal">
              Fullscreen
            </label>
            <p id="fullscreen-hint" className="text-sm text-muted-foreground">
              The game starts in fullscreen. Width and height are not used.
            </p>
          </div>
          <Switch
            id="fullscreen"
            className="shrink-0"
            aria-label="Fullscreen"
            aria-describedby="fullscreen-hint"
            checked={draft.fullscreen}
            onCheckedChange={(fullscreen) => onChange({ fullscreen })}
          />
        </div>
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            <label htmlFor="width" className="text-sm font-normal">
              Window width
            </label>
            <p id="width-hint" className="text-sm text-muted-foreground">
              Window width in pixels.
            </p>
          </div>
          <Input
            id="width"
            type="number"
            min={1}
            inputMode="numeric"
            placeholder="854"
            className="w-28 shrink-0"
            aria-describedby="width-hint"
            value={draft.width}
            disabled={draft.fullscreen}
            onChange={(event) => onChange({ width: event.target.value })}
          />
        </div>
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            <label htmlFor="height" className="text-sm font-normal">
              Window height
            </label>
            <p id="height-hint" className="text-sm text-muted-foreground">
              Window height in pixels.
            </p>
          </div>
          <Input
            id="height"
            type="number"
            min={1}
            inputMode="numeric"
            placeholder="480"
            className="w-28 shrink-0"
            aria-describedby="height-hint"
            value={draft.height}
            disabled={draft.fullscreen}
            onChange={(event) => onChange({ height: event.target.value })}
          />
        </div>
      </Card>

      <Card title="Memory" aria-label="Memory">
        <div className="flex flex-col gap-0.5">
          <div className="flex items-center justify-between gap-3">
            <span className="text-sm font-normal">Memory Allocation</span>
            <MemoryInput
              valueMb={allocationMb}
              minimumMb={snapshot.memoryMinimumMb}
              maximumMb={maximumMb}
              onChange={updateMemory}
            />
          </div>
          <p id="memory-hint" className="text-sm text-muted-foreground">
            Memory allocation. The minimum stays 512 MB.
          </p>
          {memoryPreview.totalMb !== null ? (
            <p className="text-sm text-muted-foreground">
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
