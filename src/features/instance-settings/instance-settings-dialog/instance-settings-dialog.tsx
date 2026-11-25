import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import type { InstanceBaseInput } from "@/api/commands";
import { getSettings } from "@/api/commands";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { InstanceSettingsPanel } from "@/features/instance-settings/components/instance-settings-panel";
import { InstanceSettingsSidebar } from "@/features/instance-settings/components/instance-settings-sidebar";
import { useInstanceDraft } from "@/features/instance-settings/hooks/use-instance-draft";
import type { InstanceSectionId } from "@/features/instance-settings/model/sections";
import { modpackOrigin } from "@/features/instances/lib/modpack-content";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

import { instanceSettingsDialogStyles } from "./instance-settings-dialog.styles";

export function InstanceSettingsDialog({
  entry,
  open,
  busy,
  versionBusy,
  instanceActive = false,
  onOpenChange,
  onOpenGlobalSettings,
  onRename,
  onChangeBase,
  onOpenFolder,
  onDuplicate,
  onDelete,
}: {
  entry: InstanceEntry | null;
  open: boolean;
  busy: boolean;
  versionBusy: boolean;
  /** The instance's game or a download for it is running. */
  instanceActive?: boolean;
  onOpenChange: (open: boolean) => void;
  onOpenGlobalSettings: () => void;
  onRename: (name: string) => void;
  onChangeBase: (base: InstanceBaseInput) => void;
  onOpenFolder: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  const [section, setSection] = useState<InstanceSectionId>("general");
  const origin = modpackOrigin(entry?.installation?.source);
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
            modpack={
              origin !== null
                ? { origin, linked: entry.installation?.modpack ?? null }
                : null
            }
            busy={busy}
            versionBusy={versionBusy}
            instanceActive={instanceActive}
            onChange={update}
            onRename={onRename}
            onChangeBase={onChangeBase}
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
