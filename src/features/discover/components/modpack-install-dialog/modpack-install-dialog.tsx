import { DownloadIcon, LayersIcon } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { useInstallModpack } from "@/features/discover/hooks/use-install-modpack";
import { useModpackVersions } from "@/features/discover/hooks/use-modpacks";
import { defaultVersionId } from "@/features/discover/lib/default-version";
import { formatVersionLabel } from "@/features/discover/lib/format";
import { FormSection } from "@/features/new-instance/components/form-section";
import { NameField } from "@/features/new-instance/components/name-field";
import { useInstanceNames } from "@/features/new-instance/hooks/use-instance-names";
import { uniqueName } from "@/features/new-instance/model/create-state";
import { friendlyError, messageOf } from "@/lib/errors";
import type { ModpackDetails } from "@/lib/generated/ModpackDetails";

import { modpackInstallDialogStyles } from "./modpack-install-dialog.styles";

/**
 * Picks a pack version and a name, then creates the instance. Opens on
 * `versionId` when given, otherwise on the newest release.
 */
export function ModpackInstallDialog({
  modpack,
  open,
  versionId: initialVersionId,
  onOpenChange,
}: {
  modpack: ModpackDetails;
  open: boolean;
  versionId: string | null;
  onOpenChange: (open: boolean) => void;
}) {
  const versions = useModpackVersions(modpack.source, modpack.id);
  const install = useInstallModpack(modpack);
  const takenNames = useInstanceNames();
  const suggestedName = uniqueName(modpack.name, takenNames);
  const [name, setName] = useState("");
  const [pickedVersion, setPickedVersion] = useState(initialVersionId);
  const versionId = pickedVersion ?? defaultVersionId(versions.data ?? []);

  // Every opening starts from the version it was opened for and a blank name.
  useEffect(() => {
    if (open) {
      setPickedVersion(initialVersionId);
      setName("");
    }
  }, [open, initialVersionId]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={modpackInstallDialogStyles.content}>
        <form
          className={modpackInstallDialogStyles.form}
          onSubmit={(event) => {
            event.preventDefault();
            if (versionId === null) return;
            install.mutate({ name: name.trim() || suggestedName, versionId });
          }}
        >
          <DialogHeader>
            <DialogTitle>Install {modpack.name}</DialogTitle>
            <DialogDescription>
              Creates a new instance with this modpack.
            </DialogDescription>
          </DialogHeader>
          <FormSection
            divided={false}
            icon={LayersIcon}
            title="Choose a version"
            description={
              initialVersionId === null
                ? "The newest release is selected."
                : "The version you picked is selected."
            }
          >
            {versions.isPending ? (
              <p className={modpackInstallDialogStyles.status}>
                <Spinner />
                Loading versions…
              </p>
            ) : versions.isError ? (
              <p role="alert" className={modpackInstallDialogStyles.error}>
                {friendlyError(messageOf(versions.error))}
              </p>
            ) : versions.data.length === 0 ? (
              <p className={modpackInstallDialogStyles.status}>
                This pack has no versions to install.
              </p>
            ) : (
              <Select
                value={versionId ?? undefined}
                onValueChange={setPickedVersion}
              >
                <SelectTrigger
                  aria-label="Version"
                  className={modpackInstallDialogStyles.select}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {versions.data.map((version) => (
                    <SelectItem key={version.id} value={version.id}>
                      {formatVersionLabel(version)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </FormSection>
          <NameField
            value={name}
            placeholder={suggestedName}
            onChange={setName}
          />
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">
                Cancel
              </Button>
            </DialogClose>
            <Button
              type="submit"
              disabled={versionId === null || install.isPending}
            >
              {install.isPending ? <Spinner /> : <DownloadIcon aria-hidden />}
              Install
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
