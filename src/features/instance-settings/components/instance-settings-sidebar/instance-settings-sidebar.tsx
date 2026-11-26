import { Button } from "@/components/ui/button";
import {
  INSTANCE_SECTIONS,
  type InstanceSectionId,
} from "@/features/instance-settings/model/sections";
import { InstanceIcon } from "@/features/instances/components/instance-icon";

import { instanceSettingsSidebarStyles } from "./instance-settings-sidebar.styles";

export function InstanceSettingsSidebar({
  name,
  iconPath,
  section,
  onSectionChange,
}: {
  name: string;
  iconPath: string | null;
  section: InstanceSectionId;
  onSectionChange: (section: InstanceSectionId) => void;
}) {
  return (
    <div className={instanceSettingsSidebarStyles.root}>
      <div
        data-tauri-drag-region
        aria-hidden
        className={instanceSettingsSidebarStyles.dragRegion}
      />
      <div className={instanceSettingsSidebarStyles.header}>
        <InstanceIcon iconPath={iconPath} />
        <h1 className={instanceSettingsSidebarStyles.title} title={name}>
          {name}
        </h1>
      </div>
      <nav
        aria-label="Instance settings sections"
        className={instanceSettingsSidebarStyles.nav}
      >
        {INSTANCE_SECTIONS.map((item) => {
          const Icon = item.icon;
          return (
            <Button
              key={item.id}
              type="button"
              variant={section === item.id ? "secondary" : "ghost"}
              size="sm"
              className={instanceSettingsSidebarStyles.navButton}
              aria-current={section === item.id ? "page" : undefined}
              onClick={() => onSectionChange(item.id)}
            >
              <Icon className={instanceSettingsSidebarStyles.navIcon} />
              <span className={instanceSettingsSidebarStyles.navLabel}>
                {item.label}
              </span>
            </Button>
          );
        })}
      </nav>
    </div>
  );
}
