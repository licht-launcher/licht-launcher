import { useState } from "react";

import { PageTransition } from "@/components/ui/page-transition";
import { CreateHeader } from "@/features/new-instance/components/create-header";
import { ScratchActions } from "@/features/new-instance/components/scratch-actions";
import { ScratchDetails } from "@/features/new-instance/components/scratch-details";
import { SourcePicker } from "@/features/new-instance/components/source-picker";
import { useCreateInstance } from "@/features/new-instance/hooks/use-create-instance";
import { useScratchForm } from "@/features/new-instance/hooks/use-scratch-form";
import {
  type CreateView,
  canCreateScratch,
} from "@/features/new-instance/model/create-state";

/** The popup's content: pick a source, then (for from scratch) fill the details. */
export function CreateFlow({ onClose }: { onClose: () => void }) {
  const [view, setView] = useState<CreateView>("source");
  const [direction, setDirection] = useState<"left" | "right">("left");

  const { state, update, versions, loaderVersions } = useScratchForm(
    view === "details",
  );
  const { fromScratch, fromUpload, busy } = useCreateInstance({
    scratch: state,
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
          <div className="flex flex-col gap-5">
            <ScratchDetails
              state={state}
              versions={versions.data ?? []}
              versionsPending={versions.isPending}
              versionsError={versions.isError}
              loaderVersions={loaderVersions.data ?? []}
              loaderVersionsPending={loaderVersions.isPending}
              onRetryVersions={() => {
                void versions.refetch();
              }}
              onChange={update}
            />
            <ScratchActions
              busy={busy}
              creating={fromScratch.isPending}
              canCreate={canCreateScratch(state)}
              onBack={() => {
                goTo("source");
              }}
              onCreate={() => {
                fromScratch.mutate();
              }}
            />
          </div>
        )}
      </PageTransition>
    </>
  );
}
