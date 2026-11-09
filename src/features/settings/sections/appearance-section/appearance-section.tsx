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

import { appearanceSectionStyles } from "./appearance-section.styles";

const themes = [
  {
    value: "dark",
    label: "Dark",
    icon: <MoonIcon className={appearanceSectionStyles.themeIcon} />,
  },
  {
    value: "light",
    label: "Light",
    icon: <SunIcon className={appearanceSectionStyles.themeIcon} />,
  },
  {
    value: "system",
    label: "System",
    icon: <MonitorIcon className={appearanceSectionStyles.icon} />,
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
      <div className={appearanceSectionStyles.row}>
        <div className={appearanceSectionStyles.rowText}>
          <span className={appearanceSectionStyles.rowLabel}>Language</span>
          <p
            id="language-hint"
            className={appearanceSectionStyles.rowDescription}
          >
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
        className={appearanceSectionStyles.themeField}
      >
        <div className={appearanceSectionStyles.rowText}>
          <span
            id="color-theme-label"
            className={appearanceSectionStyles.rowLabel}
          >
            Color theme
          </span>
          <p
            id="color-theme-hint"
            className={appearanceSectionStyles.rowDescription}
          >
            Choose the launcher's color theme.
          </p>
        </div>
        <div className={appearanceSectionStyles.themeOptions}>
          {themes.map(({ value, label, icon }) => (
            <ChoiceCard
              key={value}
              title={label}
              icon={icon}
              selected={draft.theme === value}
              onSelect={() => onChange({ theme: value })}
              className={appearanceSectionStyles.themeOption}
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
