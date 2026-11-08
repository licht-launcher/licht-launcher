import { CopyIcon, FolderOpenIcon, Trash2Icon } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

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
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <div className="flex flex-col gap-1">
          <label className="text-sm font-medium" htmlFor="instance-name">
            Name
          </label>
          <p id="instance-name-hint" className="text-sm text-muted-foreground">
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

      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <div className="flex flex-col gap-1">
            <h3 className="text-sm font-medium">Open folder</h3>
            <p className="text-sm text-muted-foreground">
              Open this instance directory in the system file manager.
            </p>
          </div>
          <Button
            type="button"
            variant="secondary"
            className="self-start"
            disabled={busy}
            onClick={onOpenFolder}
          >
            <FolderOpenIcon data-icon="inline-start" aria-hidden />
            Open folder
          </Button>
        </div>
        <div className="flex flex-col gap-2">
          <div className="flex flex-col gap-1">
            <h3 className="text-sm font-medium">Duplicate</h3>
            <p className="text-sm text-muted-foreground">
              Copy this instance, including world saves, under a new name.
            </p>
          </div>
          <Button
            type="button"
            variant="secondary"
            className="self-start"
            disabled={busy}
            onClick={onDuplicate}
          >
            <CopyIcon data-icon="inline-start" aria-hidden />
            Duplicate
          </Button>
        </div>
      </div>

      <div className="flex flex-col gap-2 border-t border-border pt-4">
        <div className="flex flex-col gap-1">
          <h3 className="text-sm font-medium">Delete</h3>
          <p className="text-sm text-muted-foreground">
            Permanently remove this instance and its saves. This cannot be
            undone.
          </p>
        </div>
        <Button
          type="button"
          variant="destructive"
          className="self-start"
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
