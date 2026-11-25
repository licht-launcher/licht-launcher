import { BoxIcon } from "lucide-react";
import { useEffect } from "react";
import type { InstanceBaseInput } from "@/api/commands";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Reveal } from "@/components/ui/reveal";
import { FormSection } from "@/features/new-instance/components/form-section";
import { GameVersionField } from "@/features/new-instance/components/game-version-field";
import { LoaderOptions } from "@/features/new-instance/components/loader-options";
import { PlatformPicker } from "@/features/new-instance/components/platform-picker";
import { useScratchForm } from "@/features/new-instance/hooks/use-scratch-form";
import {
  canCreateScratch,
  initialScratchState,
  type PlatformId,
  platformNeedsLoader,
  type ScratchState,
} from "@/features/new-instance/model/create-state";
import type { InstancePlatform } from "@/lib/generated/InstancePlatform";
import type { LoaderPlatform } from "@/lib/generated/LoaderPlatform";

import { editInstanceBaseDialogStyles as styles } from "./edit-instance-base-dialog.styles";

const PLATFORM_IDS: Record<LoaderPlatform, PlatformId> = {
  vanilla: "vanilla",
  fabric: "fabric",
  quilt: "quilt",
  forge: "forge",
  neoForge: "neoforge",
};

/** The form starts from the instance's base; an unknown one starts empty. */
function initialBase(current: InstancePlatform | null): ScratchState {
  if (current === null) return initialScratchState;
  return {
    ...initialScratchState,
    platform: PLATFORM_IDS[current.platform],
    gameVersion: current.gameVersion,
    // Keep the installed build until the user picks another channel.
    loaderChannel: current.loaderVersion ? "manual" : "stable",
    loaderVersion: current.loaderVersion ?? "",
  };
}

function sameBase(state: ScratchState, current: InstancePlatform | null) {
  return (
    current !== null &&
    state.platform === PLATFORM_IDS[current.platform] &&
    state.gameVersion === current.gameVersion &&
    (!platformNeedsLoader(state.platform) ||
      state.loaderVersion === (current.loaderVersion ?? ""))
  );
}

/**
 * Picks another platform, game version or loader build for an instance that is
 * not linked to a modpack.
 */
export function EditInstanceBaseDialog({
  open,
  current,
  busy,
  onOpenChange,
  onSave,
}: {
  open: boolean;
  /** The instance's base now, or `null` when it is not known. */
  current: InstancePlatform | null;
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (base: InstanceBaseInput) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={styles.content}>
        <DialogHeader>
          <DialogTitle>Edit base</DialogTitle>
          <DialogDescription>
            Choose the platform, game version and loader this instance launches
            with. Missing files are downloaded before the change is saved.
          </DialogDescription>
        </DialogHeader>
        {/* Mounted only while open, so each opening starts from the current base. */}
        <BaseForm
          current={current}
          busy={busy}
          onCancel={() => onOpenChange(false)}
          onSave={(base) => {
            onSave(base);
            onOpenChange(false);
          }}
        />
      </DialogContent>
    </Dialog>
  );
}

function BaseForm({
  current,
  busy,
  onCancel,
  onSave,
}: {
  current: InstancePlatform | null;
  busy: boolean;
  onCancel: () => void;
  onSave: (base: InstanceBaseInput) => void;
}) {
  const {
    state,
    update,
    versions,
    loaderVersions,
    loaderState,
    stableMissing,
  } = useScratchForm(initialBase(current));
  const loaderReady = loaderState === "none" || loaderState === "ready";
  // The version list hides snapshots by default; show them when the instance runs one.
  const runsSnapshot =
    versions.data?.find((version) => version.id === current?.gameVersion)
      ?.versionType === "snapshot";
  useEffect(() => {
    if (runsSnapshot) update({ includeSnapshots: true });
  }, [runsSnapshot, update]);
  const changesPlatform =
    current !== null && state.platform !== PLATFORM_IDS[current.platform];

  return (
    <>
      <div className={styles.body}>
        <FormSection
          icon={BoxIcon}
          title="Platform"
          description="Vanilla is the official game. Loaders add mod support."
        >
          <PlatformPicker
            value={state.platform}
            onChange={(platform) => {
              update({ platform, loaderChannel: "stable", loaderVersion: "" });
            }}
          />
        </FormSection>
        <GameVersionField
          state={state}
          versions={versions.data ?? []}
          pending={versions.isPending}
          error={versions.isError}
          onChange={update}
          onRetry={() => {
            void versions.refetch();
          }}
        />
        <Reveal open={platformNeedsLoader(state.platform)}>
          <div className={styles.loaderOptions}>
            <LoaderOptions
              state={state}
              loaderState={loaderState}
              stableMissing={stableMissing}
              loaderVersions={loaderVersions.data ?? []}
              onChange={update}
            />
          </div>
        </Reveal>
        {changesPlatform ? (
          <p role="note" className={styles.warning}>
            Mods made for another platform will not load after the change.
          </p>
        ) : null}
      </div>
      <DialogFooter>
        <Button
          type="button"
          variant="outline"
          disabled={busy}
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          type="button"
          disabled={
            busy ||
            !canCreateScratch(state, loaderReady) ||
            sameBase(state, current)
          }
          onClick={() => {
            onSave({
              platform: state.platform,
              gameVersion: state.gameVersion,
              loaderChannel: state.loaderChannel,
              loaderVersion:
                platformNeedsLoader(state.platform) &&
                state.loaderChannel === "manual"
                  ? state.loaderVersion
                  : null,
            });
          }}
        >
          Save
        </Button>
      </DialogFooter>
    </>
  );
}
