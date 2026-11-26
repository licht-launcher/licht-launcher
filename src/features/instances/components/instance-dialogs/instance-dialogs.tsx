import type { UseMutationResult } from "@tanstack/react-query";
import {
  type InstanceAction,
  InstanceActionDialog,
} from "@/features/instances/components/instance-action-dialog";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

type DialogMutations = {
  rename: UseMutationResult<
    InstanceEntry,
    unknown,
    { folder: string; name: string }
  >;
  remove: UseMutationResult<void, unknown, { folder: string }>;
};

export function InstanceDialogs({
  busy,
  mutations,
  action,
  draftName,
  onActionOpenChange,
  onDraftNameChange,
}: {
  busy: boolean;
  mutations: DialogMutations;
  action: InstanceAction | null;
  draftName: string;
  onActionOpenChange: (open: boolean) => void;
  onDraftNameChange: (name: string) => void;
}) {
  return (
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
        mutations.remove.mutate({ folder: action.folder });
      }}
    />
  );
}
