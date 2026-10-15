import { useMutation } from "@tanstack/react-query";
import { useRouter, useRouterState } from "@tanstack/react-router";
import {
  ArrowLeftIcon,
  ArrowRightIcon,
  RefreshCwIcon,
  SettingsIcon,
} from "lucide-react";
import { type ReactElement, useEffect, useState } from "react";
import { toast } from "sonner";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { GitHubIcon } from "@/features/settings/components/github-icon";
import {
  getUpdateCheckStatus,
  MANUAL_UPDATE_CHECK_EVENT,
  UPDATE_CHECK_STATUS_EVENT,
} from "@/features/updater/components/update-check";
import { openRepository } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import { requestOpenSettings } from "@/lib/shell-events";

export function AppContextMenu({ children }: { children: ReactElement }) {
  const router = useRouter();
  useRouterState({
    select: (state) =>
      `${String(state.location.state.__TSR_index)}:${state.location.href}`,
  });
  const canGoBack = router.history.canGoBack();
  const canGoForward =
    router.history.location.state.__TSR_index < router.history.length - 1;
  const [checking, setChecking] = useState(
    () => getUpdateCheckStatus() === "checking",
  );
  useEffect(() => {
    const updateStatus = () => {
      setChecking(getUpdateCheckStatus() === "checking");
    };
    window.addEventListener(UPDATE_CHECK_STATUS_EVENT, updateStatus);
    updateStatus();
    return () => {
      window.removeEventListener(UPDATE_CHECK_STATUS_EVENT, updateStatus);
    };
  }, []);

  const repository = useMutation({
    mutationFn: openRepository,
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  return (
    <ContextMenu>
      <ContextMenuTrigger
        asChild
        onContextMenuCapture={(event) => {
          if (
            event.target instanceof Element &&
            event.target.closest(
              'input, textarea, [contenteditable]:not([contenteditable="false"])',
            ) !== null
          ) {
            event.stopPropagation();
          }
        }}
      >
        {children}
      </ContextMenuTrigger>
      <ContextMenuContent aria-label="Application menu">
        <ContextMenuGroup>
          <ContextMenuItem
            disabled={!canGoBack}
            onSelect={() => router.history.back()}
          >
            <ArrowLeftIcon aria-hidden />
            Back
          </ContextMenuItem>
          <ContextMenuItem
            disabled={!canGoForward}
            onSelect={() => router.history.forward()}
          >
            <ArrowRightIcon aria-hidden />
            Forward
          </ContextMenuItem>
        </ContextMenuGroup>
        <ContextMenuSeparator />
        <ContextMenuGroup>
          <ContextMenuItem onSelect={() => requestOpenSettings()}>
            <SettingsIcon aria-hidden />
            Settings
          </ContextMenuItem>
          <ContextMenuItem
            disabled={checking}
            onSelect={() => {
              window.dispatchEvent(new Event(MANUAL_UPDATE_CHECK_EVENT));
            }}
          >
            <RefreshCwIcon
              aria-hidden
              className={checking ? "animate-spin" : undefined}
            />
            {checking ? "Checking for updates…" : "Check for updates"}
          </ContextMenuItem>
        </ContextMenuGroup>
        <ContextMenuSeparator />
        <ContextMenuItem
          disabled={repository.isPending}
          onSelect={() => repository.mutate()}
        >
          <GitHubIcon aria-hidden />
          Open GitHub
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
