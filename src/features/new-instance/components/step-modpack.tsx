import { PackageIcon, SearchIcon } from "lucide-react";

import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { formatModpackSource } from "@/features/new-instance/assets/platform-icons";
import type { ModpackListItem } from "@/features/new-instance/model/wizard";
import { cn } from "@/lib/utils";

export function StepModpack({
  query,
  items,
  selectedId,
  pending,
  error,
  onQueryChange,
  onSelect,
}: {
  query: string;
  items: ModpackListItem[];
  selectedId: string | null;
  pending: boolean;
  error: string | null;
  onQueryChange: (query: string) => void;
  onSelect: (item: ModpackListItem) => void;
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <div className="flex items-center gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/20 text-instance">
          <PackageIcon aria-hidden className="size-5" strokeWidth={2} />
        </span>
        <div>
          <h2 className="text-lg font-medium">Choose a modpack</h2>
          <p className="text-sm text-muted-foreground">
            Popular packs from Modrinth and CurseForge.
          </p>
        </div>
      </div>
      <Input
        value={query}
        placeholder="Search modpacks"
        aria-label="Search modpacks"
        startIcon={<SearchIcon aria-hidden />}
        onChange={(event) => {
          onQueryChange(event.target.value);
        }}
      />
      {error !== null ? (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      ) : null}
      {pending ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Spinner className="size-4" />
          Loading modpacks…
        </div>
      ) : (
        <ScrollArea className="min-h-0 flex-1 rounded-lg border border-border/60 bg-black/20">
          <ul className="flex flex-col divide-y divide-border/40">
            {items.map((item) => (
              <li key={`${item.source}-${item.id}`}>
                <button
                  type="button"
                  className={cn(
                    "flex w-full cursor-pointer items-start gap-3 px-3 py-3 text-left outline-none transition-colors",
                    "hover:bg-muted/50 focus-visible:bg-muted/50",
                    selectedId === item.id && "bg-muted",
                  )}
                  onClick={() => {
                    onSelect(item);
                  }}
                >
                  {item.iconUrl !== null ? (
                    <img
                      src={item.iconUrl}
                      alt=""
                      className="size-10 shrink-0 rounded-md bg-black/20 object-cover"
                    />
                  ) : (
                    <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-black/20 text-muted-foreground">
                      <PackageIcon
                        aria-hidden
                        className="size-5"
                        strokeWidth={2}
                      />
                    </span>
                  )}
                  <span className="min-w-0 flex-1">
                    <span className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium">
                        {item.name}
                      </span>
                      <span className="shrink-0 text-xs text-muted-foreground">
                        {formatModpackSource(item.source)}
                      </span>
                    </span>
                    <span className="line-clamp-2 text-xs text-muted-foreground">
                      {item.description || item.author}
                    </span>
                  </span>
                </button>
              </li>
            ))}
            {items.length === 0 ? (
              <li className="px-3 py-6 text-sm text-muted-foreground">
                No modpacks found.
              </li>
            ) : null}
          </ul>
        </ScrollArea>
      )}
    </div>
  );
}
