import { CopyIcon } from "lucide-react";

import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";
import { cn } from "@/lib/utils";

export function StepClone({
  instances,
  pending,
  selectedFolder,
  name,
  onSelect,
  onNameChange,
}: {
  instances: InstanceEntry[];
  pending: boolean;
  selectedFolder: string;
  name: string;
  onSelect: (instance: InstanceEntry) => void;
  onNameChange: (name: string) => void;
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <div className="flex items-center gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/20 text-instance">
          <CopyIcon aria-hidden className="size-5" strokeWidth={2} />
        </span>
        <div>
          <h2 className="text-lg font-medium">Clone an instance</h2>
          <p className="text-sm text-muted-foreground">
            Copy an existing instance, including its worlds and mods.
          </p>
        </div>
      </div>
      <label
        className="flex max-w-md flex-col gap-1 text-sm"
        htmlFor="clone-name"
      >
        Name
        <Input
          id="clone-name"
          value={name}
          autoComplete="off"
          onChange={(event) => {
            onNameChange(event.target.value);
          }}
        />
      </label>
      {pending ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Spinner className="size-4" />
          Loading instances…
        </div>
      ) : instances.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          There are no instances to clone yet.
        </p>
      ) : (
        <ScrollArea className="min-h-0 flex-1 rounded-lg border border-border/60 bg-black/20">
          <ul
            className="flex flex-col divide-y divide-border/40"
            aria-label="Instance to clone"
          >
            {instances.map((instance) => (
              <li key={instance.folder}>
                <button
                  type="button"
                  aria-pressed={instance.folder === selectedFolder}
                  className={cn(
                    "flex w-full cursor-pointer flex-col items-start gap-0.5 px-3 py-3 text-left outline-none transition-colors",
                    "hover:bg-muted/50 focus-visible:bg-muted/50",
                    instance.folder === selectedFolder && "bg-muted",
                  )}
                  onClick={() => {
                    onSelect(instance);
                  }}
                >
                  <span className="text-sm font-medium">{instance.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {instance.versionId ?? "Preparing"}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </ScrollArea>
      )}
    </div>
  );
}
