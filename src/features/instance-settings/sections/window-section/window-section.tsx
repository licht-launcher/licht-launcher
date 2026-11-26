import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { InstanceDraft } from "@/features/instance-settings/model/draft";

import { windowSectionStyles as styles } from "./window-section.styles";

export function WindowSection({
  draft,
  globalHint,
  onChange,
}: {
  draft: InstanceDraft;
  /** What the instance uses while it does not override the window. */
  globalHint: string;
  onChange: (patch: Partial<InstanceDraft>) => void;
}) {
  return (
    <>
      <div className={styles.row}>
        <div className={styles.rowText}>
          <label htmlFor="override-window" className={styles.rowLabel}>
            Customize window settings
          </label>
          <p id="override-window-hint" className={styles.rowDescription}>
            {draft.overrideWindow
              ? "Configure fullscreen and launch resolution separately for this instance."
              : globalHint}
          </p>
        </div>
        <Switch
          id="override-window"
          className={styles.control}
          aria-label="Customize window settings"
          aria-describedby="override-window-hint"
          checked={draft.overrideWindow}
          onCheckedChange={(overrideWindow) => onChange({ overrideWindow })}
        />
      </div>
      {draft.overrideWindow ? (
        <>
          <div className={styles.row}>
            <div className={styles.rowText}>
              <label htmlFor="instance-fullscreen" className={styles.rowLabel}>
                Full screen
              </label>
              <p
                id="instance-fullscreen-hint"
                className={styles.rowDescription}
              >
                The game starts in full screen. Width and height are not used.
              </p>
            </div>
            <Switch
              id="instance-fullscreen"
              className={styles.control}
              aria-label="Full screen"
              aria-describedby="instance-fullscreen-hint"
              checked={draft.fullscreen}
              onCheckedChange={(fullscreen) => onChange({ fullscreen })}
            />
          </div>
          <div className={styles.row}>
            <div className={styles.rowText}>
              <label htmlFor="instance-width" className={styles.rowLabel}>
                Width
              </label>
              <p id="instance-width-hint" className={styles.rowDescription}>
                Window width in pixels.
              </p>
            </div>
            <Input
              id="instance-width"
              type="number"
              min={1}
              inputMode="numeric"
              placeholder="854"
              className={styles.numberInput}
              aria-describedby="instance-width-hint"
              value={draft.width}
              disabled={draft.fullscreen}
              onChange={(event) => onChange({ width: event.target.value })}
            />
          </div>
          <div className={styles.row}>
            <div className={styles.rowText}>
              <label htmlFor="instance-height" className={styles.rowLabel}>
                Height
              </label>
              <p id="instance-height-hint" className={styles.rowDescription}>
                Window height in pixels.
              </p>
            </div>
            <Input
              id="instance-height"
              type="number"
              min={1}
              inputMode="numeric"
              placeholder="480"
              className={styles.numberInput}
              aria-describedby="instance-height-hint"
              value={draft.height}
              disabled={draft.fullscreen}
              onChange={(event) => onChange({ height: event.target.value })}
            />
          </div>
        </>
      ) : null}
    </>
  );
}
