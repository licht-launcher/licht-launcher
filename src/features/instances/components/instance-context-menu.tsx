import {
  CopyIcon,
  FolderOpenIcon,
  PencilIcon,
  PlayIcon,
  Settings2Icon,
  SquareIcon,
  Trash2Icon,
} from "lucide-react";
import type { ReactElement } from "react";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { PlayPhase } from "@/features/instances/hooks/use-play-session";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export type InstanceMenuAction =
  | "play"
  | "stop"
  | "configure"
  | "open-folder"
  | "rename"
  | "duplicate"
  | "delete";

export function InstanceContextMenu({
  instance,
  phase,
  installing,
  busy,
  playBlocked,
  onAction,
  children,
}: {
  instance: InstanceEntry;
  phase: PlayPhase;
  installing: boolean;
  busy: boolean;
  playBlocked: boolean;
  onAction: (action: InstanceMenuAction, instance: InstanceEntry) => void;
  children: ReactElement;
}) {
  const showStop = phase === "running" || phase === "stopping";
  const modifyingBlocked = busy || installing || phase !== "idle";

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent aria-label={`Actions for ${instance.name}`}>
        <ContextMenuGroup>
          {showStop ? (
            <ContextMenuItem
              disabled={busy || phase === "stopping"}
              onSelect={() => onAction("stop", instance)}
            >
              <SquareIcon aria-hidden />
              {phase === "stopping" ? "Stopping…" : "Stop"}
            </ContextMenuItem>
          ) : (
            <ContextMenuItem
              disabled={
                busy || playBlocked || installing || phase === "starting"
              }
              onSelect={() => onAction("play", instance)}
            >
              <PlayIcon aria-hidden />
              {installing
                ? "Installing…"
                : phase === "starting"
                  ? "Starting…"
                  : "Play"}
            </ContextMenuItem>
          )}
          <ContextMenuItem
            disabled={busy}
            onSelect={() => onAction("configure", instance)}
          >
            <Settings2Icon aria-hidden />
            Configure
          </ContextMenuItem>
          <ContextMenuItem
            disabled={busy}
            onSelect={() => onAction("open-folder", instance)}
          >
            <FolderOpenIcon aria-hidden />
            Open folder
          </ContextMenuItem>
        </ContextMenuGroup>
        <ContextMenuSeparator />
        <ContextMenuGroup>
          <ContextMenuItem
            disabled={modifyingBlocked}
            onSelect={() => onAction("rename", instance)}
          >
            <PencilIcon aria-hidden />
            Rename
          </ContextMenuItem>
          <ContextMenuItem
            disabled={modifyingBlocked}
            onSelect={() => onAction("duplicate", instance)}
          >
            <CopyIcon aria-hidden />
            Duplicate
          </ContextMenuItem>
        </ContextMenuGroup>
        <ContextMenuSeparator />
        <ContextMenuItem
          variant="destructive"
          disabled={modifyingBlocked}
          onSelect={() => onAction("delete", instance)}
        >
          <Trash2Icon aria-hidden />
          Delete
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
