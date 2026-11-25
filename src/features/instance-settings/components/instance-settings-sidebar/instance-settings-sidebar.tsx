import { ChevronRightIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { DialogTitle } from "@/components/ui/dialog";
import {
  type InstanceSectionId,
  visibleSections,
} from "@/features/instance-settings/model/sections";

import { instanceSettingsSidebarStyles } from "./instance-settings-sidebar.styles";

export function InstanceSettingsSidebar({
  name,
  section,
  linkedToModpack,
  onSectionChange,
  onOpenGlobalSettings,
}: {
  name: string;
  section: InstanceSectionId;
  linkedToModpack: boolean;
  onSectionChange: (section: InstanceSectionId) => void;
  onOpenGlobalSettings: () => void;
}) {
  return (
    <div className={instanceSettingsSidebarStyles.root}>
      <div className={instanceSettingsSidebarStyles.header}>
        <DialogTitle className={instanceSettingsSidebarStyles.title}>
          {name}
        </DialogTitle>
        <Button
          type="button"
          variant="secondary"
          size="sm"
          className={instanceSettingsSidebarStyles.instanceButton}
          onClick={onOpenGlobalSettings}
        >
          Open Global Settings
          <ChevronRightIcon data-icon="inline-end" aria-hidden />
        </Button>
      </div>
      <nav
        aria-label="Instance settings sections"
        className={instanceSettingsSidebarStyles.nav}
      >
        {visibleSections(linkedToModpack).map((item) => {
          const Icon = item.icon;
          return (
            <Button
              key={item.id}
              type="button"
              variant={section === item.id ? "secondary" : "ghost"}
              className={instanceSettingsSidebarStyles.navButton}
              aria-current={section === item.id ? "page" : undefined}
              onClick={() => onSectionChange(item.id)}
            >
              <Icon />
              {item.label}
            </Button>
          );
        })}
      </nav>
    </div>
  );
}
