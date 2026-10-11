import type { UseMutationResult } from "@tanstack/react-query";

import { InstanceSettingsDialog } from "@/features/instance-settings/instance-settings-dialog";
import {
  type InstanceAction,
  InstanceActionDialog,
} from "@/features/instances/components/instance-action-dialog";
import type { SectionId } from "@/features/settings/model/sections";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

type DialogMutations = {
  create: UseMutationResult<
    InstanceEntry,
    unknown,
    { name: string; versionId: string }
  >;
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
  versions,
  versionsPending,
  busy,
  mutations,
  configuringOpen,
  action,
  draftName,
  draftVersion,
  includeSnapshots,
  settingsOpen,
  settingsSection,
  onSettingsOpenChange,
  onConfiguringOpenChange,
  onActionOpenChange,
  onDraftNameChange,
  onDraftVersionChange,
  onIncludeSnapshotsChange,
  onOpenGlobalSettings,
  onOpenDuplicate,
  onOpenDelete,
}: {
  configuring: InstanceEntry | null;
  versions: VersionSummary[];
  versionsPending: boolean;
  busy: boolean;
  mutations: DialogMutations;
  configuringOpen: boolean;
  action: InstanceAction | null;
  draftName: string;
  draftVersion: string;
  includeSnapshots: boolean;
  settingsOpen: boolean;
  settingsSection: SectionId;
  onSettingsOpenChange: (open: boolean) => void;
  onConfiguringOpenChange: (open: boolean) => void;
  onActionOpenChange: (open: boolean) => void;
  onDraftNameChange: (name: string) => void;
  onDraftVersionChange: (versionId: string) => void;
  onIncludeSnapshotsChange: (include: boolean) => void;
  onOpenGlobalSettings: (section: SectionId) => void;
  onOpenDuplicate: () => void;
  onOpenDelete: () => void;
}) {
  return (
    <>
      <InstanceActionDialog
        action={action}
        draftName={draftName}
        draftVersion={draftVersion}
        includeSnapshots={includeSnapshots}
        versions={versions}
        versionsPending={versionsPending}
        busy={busy}
        onOpenChange={onActionOpenChange}
        onDraftNameChange={onDraftNameChange}
        onDraftVersionChange={onDraftVersionChange}
        onIncludeSnapshotsChange={onIncludeSnapshotsChange}
        onCreate={() => {
          mutations.create.mutate({
            name: draftName,
            versionId: draftVersion,
          });
        }}
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
        onChangeVersion={(versionId) => {
          if (!configuring || versionId === configuring.versionId) {
            return;
          }
          const installed =
            versions.find((version) => version.id === versionId)?.installed ??
            false;
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
