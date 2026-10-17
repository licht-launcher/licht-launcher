import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { InstanceSettingsPanel } from "@/features/instance-settings/components/instance-settings-panel";
import { InstanceSettingsSidebar } from "@/features/instance-settings/components/instance-settings-sidebar";
import { useInstanceDraft } from "@/features/instance-settings/hooks/use-instance-draft";
import type { InstanceSectionId } from "@/features/instance-settings/model/sections";
import { getSettings } from "@/lib/commands";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

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
      <DialogContent className="flex h-3/4 max-h-screen gap-0 overflow-hidden p-0 sm:max-w-4xl">
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
          <div className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-muted/30 p-6">
            <Spinner
              className="size-5 text-muted-foreground"
              aria-label="Loading instance settings"
            />
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
