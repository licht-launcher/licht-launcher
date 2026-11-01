import { FileUpIcon, FolderOpenIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function StepFile({
  filePath,
  name,
  onBrowse,
  onNameChange,
}: {
  filePath: string;
  name: string;
  onBrowse: () => void;
  onNameChange: (name: string) => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/20 text-instance">
          <FileUpIcon aria-hidden className="size-5" strokeWidth={2} />
        </span>
        <div>
          <h2 className="text-lg font-medium">Modpack file</h2>
          <p className="text-sm text-muted-foreground">
            Pick a Modrinth .mrpack or a CurseForge .zip and name the instance.
          </p>
        </div>
      </div>
      <div className="flex max-w-md flex-col gap-2 text-sm">
        <span>File</span>
        <div className="flex items-center gap-2">
          <Button type="button" variant="outline" onClick={onBrowse}>
            <FolderOpenIcon data-icon="inline-start" aria-hidden />
            Choose file
          </Button>
          <span
            className="min-w-0 flex-1 truncate text-muted-foreground"
            title={filePath}
          >
            {filePath === "" ? "No file selected" : filePath}
          </span>
        </div>
      </div>
      <label
        className="flex max-w-md flex-col gap-1 text-sm"
        htmlFor="modpack-file-name"
      >
        Name
        <Input
          id="modpack-file-name"
          value={name}
          autoComplete="off"
          onChange={(event) => {
            onNameChange(event.target.value);
          }}
        />
      </label>
    </div>
  );
}
