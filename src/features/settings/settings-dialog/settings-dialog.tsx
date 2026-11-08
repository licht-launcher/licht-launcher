import { useEffect, useState } from "react";

import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { SettingsPanel } from "@/features/settings/components/settings-panel";
import { SettingsSidebar } from "@/features/settings/components/settings-sidebar";
import { useSettingsDraft } from "@/features/settings/hooks/use-settings-draft";
import type { SectionId } from "@/features/settings/model/sections";
import { onOpenSettingsRequest } from "@/lib/shell-events";

export function SettingsDialog({
  open,
  onOpenChange,
  initialSection = "account",
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialSection?: SectionId;
}) {
  const [section, setSection] = useState<SectionId>(initialSection);
  const { draft, snapshot, update, flush } = useSettingsDraft(open);

  useEffect(() => {
    if (open) {
      setSection(initialSection);
    }
  }, [open, initialSection]);

  useEffect(() => {
    if (!open) {
      return;
    }
    return onOpenSettingsRequest(setSection);
  }, [open]);

  function handleOpenChange(next: boolean) {
    if (!next) {
      flush();
    }
    onOpenChange(next);
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        className="flex h-5/6 max-h-screen gap-0 overflow-hidden bg-popover p-0 sm:max-w-4xl"
        onInteractOutside={(event) => {
          if (
            event.target instanceof Element &&
            (event.target.closest("[data-sonner-toast]") !== null ||
              event.target.closest("[data-window-controls]") !== null)
          ) {
            event.preventDefault();
          }
        }}
      >
        <SettingsSidebar section={section} onSectionChange={setSection} />
        {draft && snapshot ? (
          <SettingsPanel
            section={section}
            draft={draft}
            snapshot={snapshot}
            onChange={update}
          />
        ) : (
          <div className="flex min-h-0 min-w-0 flex-1 items-center justify-center p-6">
            <Spinner
              className="size-5 text-muted-foreground"
              aria-label="Loading settings"
            />
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
