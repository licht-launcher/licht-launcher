import { BlocksIcon, ImageIcon, SparklesIcon, Trash2Icon } from "lucide-react";
import type { Ref } from "react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  formatContentTitle,
  formatSize,
} from "@/features/instances/lib/content-table";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import { cn } from "@/lib/utils";

export function ContentRow({
  entry,
  index,
  last,
  rowRef,
  disabled,
  onToggle,
  onRemove,
}: {
  entry: InstanceContentEntry;
  index: number;
  last: boolean;
  rowRef: Ref<HTMLTableRowElement>;
  disabled: boolean;
  onToggle: (entry: InstanceContentEntry, enabled: boolean) => void;
  onRemove: (entry: InstanceContentEntry) => void;
}) {
  return (
    <tr
      ref={rowRef}
      data-index={index}
      aria-rowindex={index + 1}
      className={cn(
        "border-b border-border/60",
        last && "border-b-0",
        index % 2 === 0 && "bg-black/10",
      )}
    >
      <td className="px-3 py-2">
        <div className="flex min-w-0 items-center gap-3">
          {entry.iconUrl ? (
            <img
              src={entry.iconUrl}
              alt=""
              className="size-8 shrink-0 rounded-md object-cover"
            />
          ) : (
            <ContentIcon kind={entry.kind} />
          )}
          <div className="min-w-0">
            <p className="truncate font-medium" title={entry.title}>
              {entry.title}
            </p>
            <p
              className="truncate text-xs text-muted-foreground"
              title={entry.name}
            >
              {entry.version ? `${entry.version} · ` : ""}
              {entry.name}
            </p>
          </div>
        </div>
      </td>
      <td className="px-3 py-2 text-xs text-muted-foreground">
        {formatSize(entry.size)}
      </td>
      <td className="px-3 py-2">
        <div className="flex items-center justify-end gap-2">
          <Switch
            data-content-action="toggle"
            checked={entry.enabled}
            disabled={disabled}
            aria-label={`${entry.enabled ? "Disable" : "Enable"} ${entry.title}`}
            onCheckedChange={(enabled) => onToggle(entry, enabled)}
          />
          <Button
            data-content-action="remove"
            type="button"
            size="icon"
            variant="ghost"
            className="text-muted-foreground opacity-60 hover:text-destructive hover:opacity-100"
            aria-label={`Remove ${formatContentTitle(entry.name)}`}
            disabled={disabled}
            onClick={() => onRemove(entry)}
          >
            <Trash2Icon aria-hidden className="size-4" />
          </Button>
        </div>
      </td>
    </tr>
  );
}

function ContentIcon({ kind }: { kind: InstanceContentKind }) {
  const contentIcon =
    kind === "mods"
      ? {
          Icon: BlocksIcon,
          wrapperClassName: "bg-accent-violet/15",
          iconClassName: "text-accent-violet",
        }
      : kind === "resourcePacks"
        ? {
            Icon: ImageIcon,
            wrapperClassName: "bg-accent-orange/15",
            iconClassName: "text-accent-orange",
          }
        : {
            Icon: SparklesIcon,
            wrapperClassName: "bg-accent-teal/15",
            iconClassName: "text-accent-teal",
          };
  const { Icon, wrapperClassName, iconClassName } = contentIcon;
  return (
    <span
      className={cn(
        "flex size-8 shrink-0 items-center justify-center rounded-md",
        wrapperClassName,
      )}
    >
      <Icon aria-hidden className={cn("size-4", iconClassName)} />
    </span>
  );
}
