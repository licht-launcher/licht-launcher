import { ChevronRightIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  INSTANCE_SECTIONS,
  type InstanceSectionId,
} from "@/features/instance-settings/model/sections";

import { instanceSettingsSidebarStyles } from "./instance-settings-sidebar.styles";

export function InstanceSettingsSidebar({
  name,
  section,
  onSectionChange,
  onOpenGlobalSettings,
}: {
  name: string;
  section: InstanceSectionId;
  onSectionChange: (section: InstanceSectionId) => void;
  onOpenGlobalSettings: () => void;
}) {
  return (
    <div className={instanceSettingsSidebarStyles.root}>
      <div
        data-tauri-drag-region
        aria-hidden
        className={instanceSettingsSidebarStyles.dragRegion}
      />
      <h1 className={instanceSettingsSidebarStyles.title}>{name}</h1>
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
      <div className={instanceSettingsSidebarStyles.footer}>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={instanceSettingsSidebarStyles.globalButton}
          onClick={onOpenGlobalSettings}
        >
          <span className={instanceSettingsSidebarStyles.navLabel}>
            Open Global Settings
          </span>
          <ChevronRightIcon data-icon="inline-end" aria-hidden />
        </Button>
      </div>
    </div>
  );
}
