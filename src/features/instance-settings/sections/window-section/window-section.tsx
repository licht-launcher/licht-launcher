import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";

export function WindowSection({
  draft,
  onChange,
}: {
  draft: InstanceDraft;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-3">
          <label htmlFor="override-window" className="text-sm">
            Customize window settings
          </label>
          <Switch
            id="override-window"
            aria-label="Customize window settings"
            aria-describedby="override-window-hint"
            checked={draft.overrideWindow}
            onCheckedChange={(overrideWindow) => onChange({ overrideWindow })}
          />
        </div>
        <p id="override-window-hint" className="text-sm text-muted-foreground">
          Configure fullscreen and launch resolution separately for this
          instance.
        </p>
      </div>
      {draft.overrideWindow ? (
        <>
          <div className="flex flex-col gap-1">
            <div className="flex items-center justify-between gap-3">
              <label htmlFor="instance-fullscreen" className="text-sm">
                Full screen
              </label>
              <Switch
                id="instance-fullscreen"
                aria-label="Full screen"
                aria-describedby="instance-fullscreen-hint"
                checked={draft.fullscreen}
                onCheckedChange={(fullscreen) => onChange({ fullscreen })}
              />
            </div>
            <p
              id="instance-fullscreen-hint"
              className="text-sm text-muted-foreground"
            >
              The game starts in full screen. Width and height are not used.
            </p>
          </div>
          <div className="flex max-w-xs flex-col gap-1">
            <label
              className="flex flex-col gap-1 text-sm"
              htmlFor="instance-width"
            >
              Width
              <Input
                id="instance-width"
                type="number"
                min={1}
                inputMode="numeric"
                placeholder="854"
                value={draft.width}
                disabled={draft.fullscreen}
                onChange={(event) => onChange({ width: event.target.value })}
              />
            </label>
          </div>
          <div className="flex max-w-xs flex-col gap-1">
            <label
              className="flex flex-col gap-1 text-sm"
              htmlFor="instance-height"
            >
              Height
              <Input
                id="instance-height"
                type="number"
                min={1}
                inputMode="numeric"
                placeholder="480"
                value={draft.height}
                disabled={draft.fullscreen}
                onChange={(event) => onChange({ height: event.target.value })}
              />
            </label>
          </div>
        </>
      ) : null}
    </div>
  );
}
