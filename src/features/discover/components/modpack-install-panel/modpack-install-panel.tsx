import { DownloadIcon, LayersIcon } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
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

import { modpackInstallPanelStyles } from "./modpack-install-panel.styles";

/** Picks a pack version and a name, then creates the instance. */
export function ModpackInstallPanel({ modpack }: { modpack: ModpackDetails }) {
  const versions = useModpackVersions(modpack.source, modpack.id);
  const install = useInstallModpack(modpack);
  const takenNames = useInstanceNames();
  const suggestedName = uniqueName(modpack.name, takenNames);
  const [name, setName] = useState("");
  const [pickedVersion, setPickedVersion] = useState<string | null>(null);
  const versionId = pickedVersion ?? defaultVersionId(versions.data ?? []);

  return (
    <Card title="Install">
      <form
        className={modpackInstallPanelStyles.form}
        onSubmit={(event) => {
          event.preventDefault();
          if (versionId === null) return;
          install.mutate({ name: name.trim() || suggestedName, versionId });
        }}
      >
        <FormSection
          divided={false}
          icon={LayersIcon}
          title="Choose a version"
          description="The newest release is selected."
        >
          {versions.isPending ? (
            <p className={modpackInstallPanelStyles.status}>
              <Spinner />
              Loading versions…
            </p>
          ) : versions.isError ? (
            <p role="alert" className={modpackInstallPanelStyles.error}>
              {friendlyError(messageOf(versions.error))}
            </p>
          ) : versions.data.length === 0 ? (
            <p className={modpackInstallPanelStyles.status}>
              This pack has no versions to install.
            </p>
          ) : (
            <Select
              value={versionId ?? undefined}
              onValueChange={setPickedVersion}
            >
              <SelectTrigger aria-label="Version">
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
        <Button
          type="submit"
          className={modpackInstallPanelStyles.submit}
          disabled={versionId === null || install.isPending}
        >
          {install.isPending ? <Spinner /> : <DownloadIcon aria-hidden />}
          Install
        </Button>
      </form>
    </Card>
  );
}
