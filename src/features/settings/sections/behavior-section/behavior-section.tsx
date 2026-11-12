import { Card } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import type { Draft } from "@/features/settings/model/draft";

import { behaviorSectionStyles } from "./behavior-section.styles";

const preferences = [
  {
    field: "hardwareAcceleration",
    label: "Use hardware acceleration when available",
    description: "Use the GPU to render the launcher. Requires a restart.",
  },
  {
    field: "minimizeOnGameStart",
    label: "Minimize launcher when Minecraft starts",
    description: "Minimize the launcher after the game starts successfully.",
  },
  {
    field: "restoreOnGameExit",
    label: "Restore launcher when Minecraft closes",
    description: "Reopen the launcher when the last running game closes.",
  },
] as const;

export function BehaviorSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  return (
    <Card title="Behavior">
      {preferences.map(({ field, label, description }) => {
        if (field === "restoreOnGameExit" && !draft.minimizeOnGameStart)
          return null;
        return (
          <div key={field} className={behaviorSectionStyles.row}>
            <div className={behaviorSectionStyles.rowText}>
              <label htmlFor={field} className={behaviorSectionStyles.rowLabel}>
                {label}
              </label>
              <p
                id={`${field}-hint`}
                className={behaviorSectionStyles.rowDescription}
              >
                {description}
              </p>
            </div>
            <Switch
              id={field}
              className={behaviorSectionStyles.control}
              aria-describedby={`${field}-hint`}
              checked={draft[field]}
              onCheckedChange={(checked) => onChange({ [field]: checked })}
            />
          </div>
        );
      })}
    </Card>
  );
}
