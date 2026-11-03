import { useState } from "react";

import { PageTransition } from "@/components/ui/page-transition";
import { CreateHeader } from "@/features/new-instance/components/create-header";
import { ScratchActions } from "@/features/new-instance/components/scratch-actions";
import { ScratchDetails } from "@/features/new-instance/components/scratch-details";
import { SourcePicker } from "@/features/new-instance/components/source-picker";
import { useCreateInstance } from "@/features/new-instance/hooks/use-create-instance";
import { useInstanceNames } from "@/features/new-instance/hooks/use-instance-names";
import { useScratchForm } from "@/features/new-instance/hooks/use-scratch-form";
import {
  type CreateView,
  canCreateScratch,
} from "@/features/new-instance/model/create-state";

/** The popup's content: pick a source, then (for from scratch) fill the details. */
export function CreateFlow({ onClose }: { onClose: () => void }) {
  const [view, setView] = useState<CreateView>("source");
  const [direction, setDirection] = useState<"left" | "right">("left");

  const takenNames = useInstanceNames();
  const {
    state,
    update,
    versions,
    loaderVersions,
    loaderState,
    stableMissing,
  } = useScratchForm(view === "details");
  const { fromScratch, fromUpload, busy } = useCreateInstance({
    scratch: state,
    takenNames,
    onCreated: onClose,
  });

  function goTo(next: CreateView) {
    setDirection(next === "details" ? "left" : "right");
    setView(next);
  }

  return (
    <>
      <CreateHeader />
      <PageTransition animationKey={view} direction={direction}>
        {view === "source" ? (
          <SourcePicker
            busy={busy}
            onScratch={() => {
              goTo("details");
            }}
            onUpload={() => {
              fromUpload.mutate();
            }}
          />
        ) : (
          <>
            {/* Only the form scrolls; the header and the actions stay in view. */}
            <div
              data-slot="create-scroll"
              className="-mx-1 min-h-0 flex-1 overflow-y-auto px-1 pt-1 pb-5"
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
                onRetryLoader={() => {
                  void loaderVersions.refetch();
                }}
                onChange={update}
              />
            </div>
            <ScratchActions
              busy={busy}
              creating={fromScratch.isPending}
              canCreate={canCreateScratch(
                state,
                takenNames,
                loaderState === "none" || loaderState === "ready",
              )}
              onBack={() => {
                goTo("source");
              }}
              onCreate={() => {
                fromScratch.mutate();
              }}
            />
          </>
        )}
      </PageTransition>
    </>
  );
}
