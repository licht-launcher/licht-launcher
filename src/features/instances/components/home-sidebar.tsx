import { PlusIcon, SettingsIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export function HomeSidebar({
  instances,
  selectedFolder,
  busy,
  loading,
  onSelect,
  onCreate,
  onOpenSettings,
}: {
  instances: InstanceEntry[];
  selectedFolder: string | null;
  busy: boolean;
  loading: boolean;
  onSelect: (folder: string) => void;
  onCreate: () => void;
  onOpenSettings: () => void;
}) {
  return (
    <div className="flex h-full w-72 shrink-0 flex-col gap-6 overflow-hidden bg-black/20 px-4 pt-8 pb-10">
      <div className="flex flex-col gap-0.5 px-2">
        <h1 className="text-base font-medium">Licht Launcher</h1>
        <p className="text-xs text-muted-foreground">0.1.0 · Lilie</p>
      </div>

      <nav
        aria-label="Instances"
        className="flex min-h-0 flex-1 flex-col gap-2"
      >
        <h2 className="px-2 text-xs font-medium text-muted-foreground">
          Instances
        </h2>
        {loading ? (
          <p className="px-2 text-sm text-muted-foreground">Loading…</p>
        ) : null}
        {!loading && instances.length === 0 ? (
          <p className="px-2 text-sm text-muted-foreground">
            No instances yet.
          </p>
        ) : null}
        {!loading && instances.length > 0 ? (
          <ScrollArea className="min-h-0 flex-1">
            <ul className="flex flex-col gap-1 pr-3">
              {instances.map((instance) => {
                const selected = instance.folder === selectedFolder;
                return (
                  <li key={instance.folder}>
                    <Button
                      type="button"
                      variant={selected ? "secondary" : "ghost"}
                      className="w-full justify-start px-3"
                      aria-current={selected ? "page" : undefined}
                      disabled={busy}
                      onClick={() => {
                        onSelect(instance.folder);
                      }}
                    >
                      <span className="min-w-0 truncate">{instance.name}</span>
                    </Button>
                  </li>
                );
              })}
            </ul>
          </ScrollArea>
        ) : null}

        <Button
          type="button"
          variant="secondary"
          className="w-full justify-start px-3"
          disabled={busy}
          onClick={onCreate}
        >
          <PlusIcon data-icon="inline-start" aria-hidden />
          New instance
        </Button>

        <Button
          type="button"
          variant="ghost"
          className="w-full justify-start px-3"
          aria-label="Settings"
          onClick={onOpenSettings}
        >
          <SettingsIcon data-icon="inline-start" aria-hidden />
          Settings
        </Button>
      </nav>

      <p className="mt-auto px-3 text-xs leading-relaxed text-muted-foreground">
        Unofficial, not affiliated with Mojang Studios or Microsoft.
      </p>
    </div>
  );
}
