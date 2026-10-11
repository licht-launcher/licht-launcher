import { TagIcon } from "lucide-react";

import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import type { ModpackVersionItem } from "@/features/new-instance/model/wizard";
import { cn } from "@/lib/utils";

export function StepModpackVersion({
  name,
  versions,
  versionId,
  versionsPending,
  onNameChange,
  onVersionChange,
}: {
  name: string;
  versions: ModpackVersionItem[];
  versionId: string;
  versionsPending: boolean;
  onNameChange: (name: string) => void;
  onVersionChange: (versionId: string) => void;
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <div className="flex items-center gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/25 text-instance">
          <TagIcon aria-hidden className="size-5" strokeWidth={2} />
        </span>
        <div>
          <h2 className="text-lg font-medium">Pack version</h2>
          <p className="text-sm text-muted-foreground">
            Pick a pack build and the instance name.
          </p>
        </div>
      </div>
      <label
        className="flex max-w-md flex-col gap-1 text-sm"
        htmlFor="modpack-name"
      >
        Name
        <Input
          id="modpack-name"
          value={name}
          autoComplete="off"
          onChange={(event) => {
            onNameChange(event.target.value);
          }}
        />
      </label>
      {versionsPending ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Spinner className="size-4" />
          Loading pack versions…
        </div>
      ) : versions.length === 0 ? (
        <p className="text-sm text-muted-foreground">No pack versions found.</p>
      ) : (
        <ScrollArea className="min-h-0 flex-1 rounded-lg border border-border/60 bg-black/20">
          <ul
            className="flex flex-col divide-y divide-border/40"
            aria-label="Modpack version"
          >
            {versions.map((version) => {
              const selected = version.id === versionId;
              const meta = [
                version.gameVersions.slice(0, 3).join(", "),
                version.loaders.slice(0, 2).join(", "),
              ]
                .filter((part) => part !== "")
                .join(" · ");
              return (
                <li key={version.id}>
                  <button
                    type="button"
                    aria-pressed={selected}
                    className={cn(
                      "flex w-full cursor-pointer flex-col items-start gap-0.5 px-3 py-3 text-left outline-none transition-colors",
                      "hover:bg-muted/50 focus-visible:bg-muted/50",
                      selected && "bg-muted",
                    )}
                    onClick={() => {
                      onVersionChange(version.id);
                    }}
                  >
                    <span className="text-sm font-medium">{version.name}</span>
                    {meta !== "" ? (
                      <span className="text-xs text-muted-foreground">
                        {meta}
                      </span>
                    ) : null}
                  </button>
                </li>
              );
            })}
          </ul>
        </ScrollArea>
      )}
    </div>
  );
}
