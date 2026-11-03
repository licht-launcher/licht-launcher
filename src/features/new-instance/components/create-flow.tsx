import { CreateHeader } from "@/features/new-instance/components/create-header";
import { ScratchActions } from "@/features/new-instance/components/scratch-actions";
import { ScratchDetails } from "@/features/new-instance/components/scratch-details";
import { useCreateInstance } from "@/features/new-instance/hooks/use-create-instance";
import { useInstanceNames } from "@/features/new-instance/hooks/use-instance-names";
import { useScratchForm } from "@/features/new-instance/hooks/use-scratch-form";
import { canCreateScratch } from "@/features/new-instance/model/create-state";

/** The popup's content: the from-scratch form, with its actions kept in view. */
export function CreateFlow({ onClose }: { onClose: () => void }) {
  const takenNames = useInstanceNames();
  const {
    state,
    update,
    versions,
    loaderVersions,
    loaderState,
    stableMissing,
  } = useScratchForm();
  const create = useCreateInstance({
    scratch: state,
    takenNames,
    onCreated: onClose,
  });

  return (
    <>
      <CreateHeader />
      {/* Only the form scrolls; the header and the actions stay in view. */}
      <div
        data-slot="create-scroll"
        className="-mx-1 min-h-0 flex-1 overflow-y-auto px-1"
      >
        <ScratchDetails
          state={state}
          takenNames={takenNames}
          loaderState={loaderState}
          stableMissing={stableMissing}
          versions={versions.data ?? []}
          versionsPending={versions.isPending}
          versionsError={versions.isError}
          loaderVersions={loaderVersions.data ?? []}
          onRetryVersions={() => {
            void versions.refetch();
          }}
          onChange={update}
        />
      </div>
      <ScratchActions
        creating={create.isPending}
        canCreate={canCreateScratch(
          state,
          loaderState === "none" || loaderState === "ready",
        )}
        onCancel={onClose}
        onCreate={() => {
          create.mutate();
        }}
      />
    </>
  );
}
