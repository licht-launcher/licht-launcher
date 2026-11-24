import { Trash2Icon } from "lucide-react";
import type { Ref } from "react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { ContentKindIcon } from "@/features/instances/components/content/content-kind-icon";
import { formatContentTitle } from "@/features/instances/lib/content-table";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import { cn } from "@/lib/utils";

import { contentRowStyles } from "./content-row.styles";

export function ContentRow({
  entry,
  index,
  last,
  rowRef,
  actions,
}: {
  entry: InstanceContentEntry;
  index: number;
  last: boolean;
  rowRef: Ref<HTMLTableRowElement>;
  /** Omitted for read-only lists, such as content a modpack manages. */
  actions?: ContentActions;
}) {
  return (
    <tr
      ref={rowRef}
      data-index={index}
      aria-rowindex={index + 1}
      className={cn(
        contentRowStyles.row,
        last && contentRowStyles.rowLast,
        index % 2 === 0 && contentRowStyles.rowOdd,
      )}
    >
      <td className={contentRowStyles.cell}>
        <div className={contentRowStyles.item}>
          {entry.iconUrl ? (
            <img
              src={entry.iconUrl}
              alt=""
              className={contentRowStyles.image}
            />
          ) : (
            <ContentKindIcon kind={entry.kind} />
          )}
          <div className={contentRowStyles.text}>
            <p className={contentRowStyles.name} title={entry.title}>
              {entry.title}
            </p>
            <p className={contentRowStyles.detail} title={entry.name}>
              {entry.version ? `${entry.version} · ` : ""}
              {entry.name}
            </p>
          </div>
        </div>
      </td>
      {actions ? <ContentRowActions entry={entry} {...actions} /> : null}
    </tr>
  );
}

export type ContentActions = {
  disabled: boolean;
  onToggle: (entry: InstanceContentEntry, enabled: boolean) => void;
  /** Omitted where content can be disabled but not deleted. */
  onRemove?: (entry: InstanceContentEntry) => void;
};

function ContentRowActions({
  entry,
  disabled,
  onToggle,
  onRemove,
}: ContentActions & { entry: InstanceContentEntry }) {
  return (
    <td className={contentRowStyles.cell}>
      <div className={contentRowStyles.actions}>
        <Switch
          data-content-action="toggle"
          checked={entry.enabled}
          disabled={disabled}
          aria-label={`${entry.enabled ? "Disable" : "Enable"} ${entry.title}`}
          onCheckedChange={(enabled) => onToggle(entry, enabled)}
        />
        {onRemove ? (
          <Button
            data-content-action="remove"
            type="button"
            size="icon"
            variant="ghost"
            className={contentRowStyles.removeButton}
            aria-label={`Remove ${formatContentTitle(entry.name)}`}
            disabled={disabled}
            onClick={() => onRemove(entry)}
          >
            <Trash2Icon aria-hidden className={contentRowStyles.icon} />
          </Button>
        ) : null}
      </div>
    </td>
  );
}
