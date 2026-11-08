import { ChevronRightIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { DialogTitle } from "@/components/ui/dialog";
import {
  INSTANCE_SECTIONS,
  type InstanceSectionId,
} from "@/features/instance-settings/model/sections";

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
    <div className="flex w-56 shrink-0 flex-col gap-4 bg-black/20 px-2 py-6">
      <div className="flex flex-col gap-2 px-2">
        <DialogTitle className="border border-transparent pl-2 text-lg">
          {name}
        </DialogTitle>
        <Button
          type="button"
          variant="secondary"
          size="sm"
          className="w-full justify-between"
          onClick={onOpenGlobalSettings}
        >
          Open Global Settings
          <ChevronRightIcon data-icon="inline-end" aria-hidden />
        </Button>
      </div>
      <nav
        aria-label="Instance settings sections"
        className="flex flex-col gap-1"
      >
        {INSTANCE_SECTIONS.map((item) => {
          const Icon = item.icon;
          return (
            <Button
              key={item.id}
              type="button"
              variant={section === item.id ? "secondary" : "ghost"}
              className="justify-start px-4"
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
