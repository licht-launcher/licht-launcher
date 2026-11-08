import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { getSettings } from "@/api/commands";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { InstanceSettingsPanel } from "@/features/instance-settings/components/instance-settings-panel";
import { InstanceSettingsSidebar } from "@/features/instance-settings/components/instance-settings-sidebar";
import { useInstanceDraft } from "@/features/instance-settings/hooks/use-instance-draft";
import type { InstanceSectionId } from "@/features/instance-settings/model/sections";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

import { instanceSettingsDialogStyles } from "./instance-settings-dialog.styles";

export function InstanceSettingsDialog({
  entry,
  open,
  busy,
  versionBusy,
  onOpenChange,
  onOpenGlobalSettings,
  onRename,
  onChangeVersion,
  onOpenFolder,
  onDuplicate,
  onDelete,
}: {
  entry: InstanceEntry | null;
  open: boolean;
  busy: boolean;
  versionBusy: boolean;
  onOpenChange: (open: boolean) => void;
  onOpenGlobalSettings: () => void;
  onRename: (name: string) => void;
  onChangeVersion: (versionId: string, installed: boolean) => void;
  onOpenFolder: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  const [section, setSection] = useState<InstanceSectionId>("general");
  const { draft, update, flush } = useInstanceDraft(entry, open);
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    enabled: open,
    retry: false,
  });

  function handleOpenChange(next: boolean) {
    if (!next) {
      flush();
    }
    onOpenChange(next);
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className={instanceSettingsDialogStyles.content}>
        {entry ? (
          <InstanceSettingsSidebar
            name={entry.name}
            section={section}
            onSectionChange={setSection}
            onOpenGlobalSettings={onOpenGlobalSettings}
          />
        ) : null}
        {entry && draft && settings.data ? (
          <InstanceSettingsPanel
            section={section}
            draft={draft}
            snapshot={settings.data}
            instanceName={entry.name}
            folder={entry.folder}
            versionId={entry.versionId}
            preparing={
              entry.installation !== undefined &&
              entry.installation.state !== "ready"
            }
            busy={busy}
            versionBusy={versionBusy}
            onChange={update}
            onRename={onRename}
            onChangeVersion={onChangeVersion}
            onOpenFolder={onOpenFolder}
            onDuplicate={onDuplicate}
            onDelete={onDelete}
          />
        ) : (
          <div className={instanceSettingsDialogStyles.loading}>
            <Spinner
              className={instanceSettingsDialogStyles.spinner}
              aria-label="Loading instance settings"
            />
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
