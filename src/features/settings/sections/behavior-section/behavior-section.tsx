import { Card } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import type { Draft } from "@/features/settings/model/draft";

const preferences = [
  {
    field: "hardwareAcceleration",
    label: "Use hardware acceleration when available",
    description: "Use the GPU to render the launcher. Requires a restart.",
  },
  {
    field: "minimizeToTray",
    label: "Minimize to tray on close",
    description:
      "Keep the launcher running in the system tray when closing its window.",
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
          <div key={field} className="flex items-center justify-between gap-3">
            <div className="flex min-w-0 flex-col gap-0.5">
              <label htmlFor={field} className="text-sm font-normal">
                {label}
              </label>
              <p id={`${field}-hint`} className="text-sm text-muted-foreground">
                {description}
              </p>
            </div>
            <Switch
              id={field}
              className="shrink-0"
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
