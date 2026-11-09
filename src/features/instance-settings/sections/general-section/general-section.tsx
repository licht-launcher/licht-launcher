import { CopyIcon, FolderOpenIcon, Trash2Icon } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

import { generalSectionStyles } from "./general-section.styles";

export function GeneralSection({
  name,
  busy,
  onRename,
  onOpenFolder,
  onDuplicate,
  onDelete,
}: {
  name: string;
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
      <div className={generalSectionStyles.group}>
        <div className={generalSectionStyles.groupText}>
          <label className={generalSectionStyles.label} htmlFor="instance-name">
            Name
          </label>
          <p id="instance-name-hint" className={generalSectionStyles.hint}>
            Display name for this instance. The folder on disk stays the same.
          </p>
        </div>
        <Input
          id="instance-name"
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

      <div className={generalSectionStyles.actions}>
        <div className={generalSectionStyles.group}>
          <div className={generalSectionStyles.groupText}>
            <h3 className={generalSectionStyles.label}>Open folder</h3>
            <p className={generalSectionStyles.hint}>
              Open this instance directory in the system file manager.
            </p>
          </div>
          <Button
            type="button"
            variant="secondary"
            className={generalSectionStyles.action}
            disabled={busy}
            onClick={onOpenFolder}
          >
            <FolderOpenIcon data-icon="inline-start" aria-hidden />
            Open folder
          </Button>
        </div>
        <div className={generalSectionStyles.group}>
          <div className={generalSectionStyles.groupText}>
            <h3 className={generalSectionStyles.label}>Duplicate</h3>
            <p className={generalSectionStyles.hint}>
              Copy this instance, including world saves, under a new name.
            </p>
          </div>
          <Button
            type="button"
            variant="secondary"
            className={generalSectionStyles.action}
            disabled={busy}
            onClick={onDuplicate}
          >
            <CopyIcon data-icon="inline-start" aria-hidden />
            Duplicate
          </Button>
        </div>
      </div>

      <div className={generalSectionStyles.dangerGroup}>
        <div className={generalSectionStyles.groupText}>
          <h3 className={generalSectionStyles.label}>Delete</h3>
          <p className={generalSectionStyles.hint}>
            Permanently remove this instance and its saves. This cannot be
            undone.
          </p>
        </div>
        <Button
          type="button"
          variant="destructive"
          className={generalSectionStyles.action}
          disabled={busy}
          onClick={onDelete}
        >
          <Trash2Icon data-icon="inline-start" aria-hidden />
          Delete instance
        </Button>
      </div>
    </div>
  );
}
