import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { Card } from "@/components/ui/card";
import { ChoiceCard } from "@/components/ui/choice-card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ThemePreview } from "@/components/ui/theme-preview";
import type { Draft } from "@/features/settings/model/draft";
import type { LauncherTheme } from "@/lib/generated/LauncherTheme";

const themes = [
  { value: "dark", label: "Dark", icon: <MoonIcon className="size-4.5" /> },
  { value: "light", label: "Light", icon: <SunIcon className="size-4.5" /> },
  {
    value: "system",
    label: "System",
    icon: <MonitorIcon className="size-4" />,
  },
] as const satisfies ReadonlyArray<{
  value: LauncherTheme;
  label: string;
  icon: React.ReactNode;
}>;

export function AppearanceSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  return (
    <Card title="Appearance">
      <div className="flex items-center justify-between gap-3 text-sm">
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="font-normal">Language</span>
          <p id="language-hint" className="text-sm text-muted-foreground">
            Choose the launcher's display language.
          </p>
        </div>
        <Select
          value={draft.language}
          onValueChange={(language) => {
            if (language === "en") onChange({ language });
          }}
        >
          <SelectTrigger aria-label="Language" aria-describedby="language-hint">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="en">English</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <fieldset
        aria-labelledby="color-theme-label"
        aria-describedby="color-theme-hint"
        className="flex min-w-0 flex-col gap-3 text-sm"
      >
        <div className="flex min-w-0 flex-col gap-0.5">
          <span id="color-theme-label" className="font-normal">
            Color theme
          </span>
          <p id="color-theme-hint" className="text-sm text-muted-foreground">
            Choose the launcher's color theme.
          </p>
        </div>
        <div className="flex gap-3">
          {themes.map(({ value, label, icon }) => (
            <ChoiceCard
              key={value}
              title={label}
              icon={icon}
              selected={draft.theme === value}
              onSelect={() => onChange({ theme: value })}
              className="min-w-0 flex-1"
            >
              <span aria-hidden="true">
                <ThemePreview theme={value} />
              </span>
            </ChoiceCard>
          ))}
        </div>
      </fieldset>
    </Card>
  );
}
