import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";

import { windowSectionStyles } from "./window-section.styles";

export function WindowSection({
  draft,
  onChange,
}: {
  draft: InstanceDraft;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  return (
    <div className={windowSectionStyles.root}>
      <div className={windowSectionStyles.toggle}>
        <div className={windowSectionStyles.toggleRow}>
          <label
            htmlFor="override-window"
            className={windowSectionStyles.label}
          >
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
        <p id="override-window-hint" className={windowSectionStyles.hint}>
          Configure fullscreen and launch resolution separately for this
          instance.
        </p>
      </div>
      {draft.overrideWindow ? (
        <>
          <div className={windowSectionStyles.toggle}>
            <div className={windowSectionStyles.toggleRow}>
              <label
                htmlFor="instance-fullscreen"
                className={windowSectionStyles.label}
              >
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
              className={windowSectionStyles.hint}
            >
              The game starts in full screen. Width and height are not used.
            </p>
          </div>
          <div className={windowSectionStyles.sizeField}>
            <label
              className={windowSectionStyles.field}
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
          <div className={windowSectionStyles.sizeField}>
            <label
              className={windowSectionStyles.field}
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
