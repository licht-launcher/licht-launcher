import type { UseMutationResult } from "@tanstack/react-query";

import { InstanceSettingsDialog } from "@/features/instance-settings/instance-settings-dialog";
import {
  type InstanceAction,
  InstanceActionDialog,
} from "@/features/instances/components/instance-action-dialog";
import type { SectionId } from "@/features/settings/model/sections";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

type DialogMutations = {
  rename: UseMutationResult<
    InstanceEntry,
    unknown,
    { folder: string; name: string }
  >;
  duplicate: UseMutationResult<
    InstanceEntry,
    unknown,
    { folder: string; name: string }
  >;
  remove: UseMutationResult<void, unknown, string>;
  openFolder: UseMutationResult<void, unknown, string>;
  changeVersion: UseMutationResult<
    InstanceEntry,
    unknown,
    { folder: string; versionId: string; installed: boolean }
  >;
};

export function InstanceDialogs({
  configuring,
  busy,
  mutations,
  configuringOpen,
  action,
  draftName,
  settingsOpen,
  settingsSection,
  onSettingsOpenChange,
  onConfiguringOpenChange,
  onActionOpenChange,
  onDraftNameChange,
  onOpenGlobalSettings,
  onOpenDuplicate,
  onOpenDelete,
}: {
  configuring: InstanceEntry | null;
  busy: boolean;
  mutations: DialogMutations;
  configuringOpen: boolean;
  action: InstanceAction | null;
  draftName: string;
  settingsOpen: boolean;
  settingsSection: SectionId;
  onSettingsOpenChange: (open: boolean) => void;
  onConfiguringOpenChange: (open: boolean) => void;
  onActionOpenChange: (open: boolean) => void;
  onDraftNameChange: (name: string) => void;
  onOpenGlobalSettings: (section: SectionId) => void;
  onOpenDuplicate: () => void;
  onOpenDelete: () => void;
}) {
  return (
    <>
      <InstanceActionDialog
        action={action}
        draftName={draftName}
        busy={busy}
        onOpenChange={onActionOpenChange}
        onDraftNameChange={onDraftNameChange}
        onDuplicate={() => {
          if (action?.kind !== "duplicate") {
            return;
          }
          mutations.duplicate.mutate({
            folder: action.folder,
            name: draftName,
          });
        }}
        onDelete={() => {
          if (action?.kind !== "delete") {
            return;
          }
          mutations.remove.mutate(action.folder);
        }}
      />

      <InstanceSettingsDialog
        entry={configuring}
        open={configuringOpen}
        busy={busy}
        versionBusy={mutations.changeVersion.isPending}
        onOpenChange={onConfiguringOpenChange}
        onOpenGlobalSettings={() => onOpenGlobalSettings("instances")}
        onRename={(name) => {
          if (!configuring) {
            return;
          }
          mutations.rename.mutate({ folder: configuring.folder, name });
        }}
        onChangeVersion={(versionId, installed) => {
          if (!configuring || versionId === configuring.versionId) {
            return;
          }
          mutations.changeVersion.mutate({
            folder: configuring.folder,
            versionId,
            installed,
          });
        }}
        onOpenFolder={() => {
          if (!configuring) {
            return;
          }
          mutations.openFolder.mutate(configuring.folder);
        }}
        onDuplicate={onOpenDuplicate}
        onDelete={onOpenDelete}
      />

      <SettingsDialog
        open={settingsOpen}
        onOpenChange={onSettingsOpenChange}
        initialSection={settingsSection}
      />
    </>
  );
}
