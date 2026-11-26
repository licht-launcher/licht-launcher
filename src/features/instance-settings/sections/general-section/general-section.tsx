import { CopyIcon, FolderOpenIcon, Trash2Icon } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { InstanceIconSettings } from "@/features/instance-settings/components/instance-icon-settings";

import { generalSectionStyles } from "./general-section.styles";

export function GeneralSection({
  name,
  folder,
  iconPath,
  customIcon,
  busy,
  onRename,
  onOpenFolder,
  onDuplicate,
  onDelete,
}: {
  name: string;
  folder: string;
  iconPath: string | null;
  customIcon: boolean;
  busy: boolean;
  onRename: (name: string) => void;
  onOpenFolder: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  const [draftName, setDraftName] = useState(name);

  useEffect(() => {
    setDraftName(name);
  }, [name]);

  function commitName() {
    const next = draftName.trim();
    if (next === "" || next === name) {
      setDraftName(name);
      return;
    }
    onRename(next);
  }

  return (
    <div className={generalSectionStyles.root}>
      <Card title="Instance">
        <InstanceIconSettings
          folder={folder}
          iconPath={iconPath}
          customIcon={customIcon}
          busy={busy}
        />
        <div className={generalSectionStyles.row}>
          <div className={generalSectionStyles.rowText}>
            <label
              className={generalSectionStyles.rowLabel}
              htmlFor="instance-name"
            >
              Name
            </label>
            <p
              id="instance-name-hint"
              className={generalSectionStyles.rowDescription}
            >
              Display name for this instance. The folder on disk stays the same.
            </p>
          </div>
          <Input
            id="instance-name"
            className={generalSectionStyles.nameInput}
            value={draftName}
            disabled={busy}
            aria-describedby="instance-name-hint"
            onChange={(event) => {
              setDraftName(event.target.value);
            }}
            onBlur={commitName}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.currentTarget.blur();
              }
            }}
          />
        </div>
      </Card>

      <Card title="Actions">
        <div className={generalSectionStyles.row}>
          <div className={generalSectionStyles.rowText}>
            <p className={generalSectionStyles.rowLabel}>Open folder</p>
            <p className={generalSectionStyles.rowDescription}>
              Open this instance directory in the system file manager.
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            className={generalSectionStyles.control}
            disabled={busy}
            onClick={onOpenFolder}
          >
            <FolderOpenIcon data-icon="inline-start" aria-hidden />
            Open folder
          </Button>
        </div>
        <div className={generalSectionStyles.row}>
          <div className={generalSectionStyles.rowText}>
            <p className={generalSectionStyles.rowLabel}>Duplicate</p>
            <p className={generalSectionStyles.rowDescription}>
              Copy this instance, including world saves, under a new name.
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            className={generalSectionStyles.control}
            disabled={busy}
            onClick={onDuplicate}
          >
            <CopyIcon data-icon="inline-start" aria-hidden />
            Duplicate
          </Button>
        </div>
      </Card>

      <Card title="Danger zone">
        <div className={generalSectionStyles.row}>
          <div className={generalSectionStyles.rowText}>
            <p className={generalSectionStyles.rowLabel}>Delete</p>
            <p className={generalSectionStyles.rowDescription}>
              Permanently remove this instance and its saves. This cannot be
              undone.
            </p>
          </div>
          <Button
            type="button"
            variant="destructive"
            className={generalSectionStyles.control}
            disabled={busy}
            onClick={onDelete}
          >
            <Trash2Icon data-icon="inline-start" aria-hidden />
            Delete instance
          </Button>
        </div>
      </Card>
    </div>
  );
}
