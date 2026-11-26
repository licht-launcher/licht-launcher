import type { UseMutationResult } from "@tanstack/react-query";
import type { InstanceBaseInput } from "@/api/commands";
import { InstanceSettingsDialog } from "@/features/instance-settings/instance-settings-dialog";
import {
  type InstanceAction,
  InstanceActionDialog,
} from "@/features/instances/components/instance-action-dialog";
import type { SectionId } from "@/features/settings/model/sections";
import type { DownloadJobSnapshot } from "@/lib/generated/DownloadJobSnapshot";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

type DialogMutations = {
  rename: UseMutationResult<
    InstanceEntry,
    unknown,
    { folder: string; name: string }
  >;
  remove: UseMutationResult<void, unknown, string>;
  openFolder: UseMutationResult<void, unknown, string>;
  changeBase: UseMutationResult<
    DownloadJobSnapshot,
    unknown,
    { folder: string; base: InstanceBaseInput }
  >;
};

export function InstanceDialogs({
  configuring,
  configuringActive,
  busy,
  mutations,
  configuringOpen,
  action,
  draftName,
  onConfiguringOpenChange,
  onActionOpenChange,
  onDraftNameChange,
  onOpenGlobalSettings,
  onOpenDuplicate,
  onOpenDelete,
}: {
  configuring: InstanceEntry | null;
  /** The configured instance's game or a download for it is running. */
  configuringActive: boolean;
  busy: boolean;
  mutations: DialogMutations;
  configuringOpen: boolean;
  action: InstanceAction | null;
  draftName: string;
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
        onRename={() => {
          if (action?.kind !== "rename") {
            return;
          }
          mutations.rename.mutate({
            folder: action.folder,
            name: draftName.trim(),
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
        versionBusy={mutations.changeBase.isPending}
        instanceActive={configuringActive}
        onOpenChange={onConfiguringOpenChange}
        onOpenGlobalSettings={() => onOpenGlobalSettings("instances")}
        onRename={(name) => {
          if (!configuring) {
            return;
          }
          mutations.rename.mutate({ folder: configuring.folder, name });
        }}
        onChangeBase={(base) => {
          if (!configuring) {
            return;
          }
          mutations.changeBase.mutate({ folder: configuring.folder, base });
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
    </>
  );
}
