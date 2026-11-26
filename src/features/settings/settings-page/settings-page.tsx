import { Spinner } from "@/components/ui/spinner";
import { SettingsPanel } from "@/features/settings/components/settings-panel";
import { SettingsSidebar } from "@/features/settings/components/settings-sidebar";
import { useSettingsDraft } from "@/features/settings/hooks/use-settings-draft";
import type { SectionId } from "@/features/settings/model/sections";

import { settingsPageStyles } from "./settings-page.styles";

export function SettingsPage({
  section,
  onSectionChange,
}: {
  section: SectionId;
  onSectionChange: (section: SectionId) => void;
}) {
  const { draft, snapshot, update } = useSettingsDraft();

  return (
    <section aria-label="Settings" className={settingsPageStyles.root}>
      <SettingsSidebar section={section} onSectionChange={onSectionChange} />
      {draft && snapshot ? (
        <SettingsPanel
          section={section}
          draft={draft}
          snapshot={snapshot}
          onChange={update}
        />
      ) : (
        <div className={settingsPageStyles.loading}>
          <Spinner
            className={settingsPageStyles.spinner}
            aria-label="Loading settings"
          />
        </div>
      )}
    </section>
  );
}
