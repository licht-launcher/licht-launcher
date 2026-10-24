import { useMutation } from "@tanstack/react-query";
import { useRouter, useRouterState } from "@tanstack/react-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ArrowLeftIcon,
  ArrowRightIcon,
  ChevronDownIcon,
  ChevronUpIcon,
  CompassIcon,
  LibraryIcon,
  LogOutIcon,
  MessageSquareIcon,
  SettingsIcon,
  UsersIcon,
} from "lucide-react";
import { useRef, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { SectionId } from "@/features/settings/model/sections";
import { openFeedback } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function LauncherNavigation({
  libraryActive,
  onOpenLibrary,
  onOpenSettings,
}: {
  libraryActive: boolean;
  onOpenLibrary: () => void;
  onOpenSettings: (section: SectionId) => void;
}) {
  const router = useRouter();
  useRouterState({
    select: (state) =>
      `${String(state.location.state.__TSR_index)}:${state.location.href}`,
  });
  const canGoBack = router.history.canGoBack();
  const canGoForward =
    router.history.location.state.__TSR_index < router.history.length - 1;
  const [menuOpen, setMenuOpen] = useState(false);
  const openingSettings = useRef(false);
  const exit = useMutation({
    mutationFn: () => getCurrentWindow().close(),
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const feedback = useMutation({
    mutationFn: openFeedback,
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });

  function openSettings(section: SectionId) {
    openingSettings.current = true;
    onOpenSettings(section);
  }

  return (
    <div className="flex shrink-0 items-center gap-1 px-2">
      <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mr-1 h-7 shrink-0 gap-1 px-1.5 font-brand text-xs font-medium tracking-widest text-muted-foreground uppercase select-none hover:bg-transparent dark:hover:bg-transparent aria-expanded:bg-transparent"
          >
            Licht Launcher
            {menuOpen ? (
              <ChevronUpIcon aria-hidden className="size-3" />
            ) : (
              <ChevronDownIcon aria-hidden className="size-3" />
            )}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          aria-label="Licht menu"
          align="start"
          className="font-brand"
          onCloseAutoFocus={(event) => {
            if (openingSettings.current) {
              event.preventDefault();
              openingSettings.current = false;
            }
          }}
        >
          <DropdownMenuItem onSelect={() => openSettings("account")}>
            <UsersIcon aria-hidden />
            Manage accounts
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => openSettings("appearance")}>
            <SettingsIcon aria-hidden />
            Settings
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            disabled={feedback.isPending}
            onSelect={() => feedback.mutate()}
          >
            <MessageSquareIcon aria-hidden />
            Send feedback
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            disabled={exit.isPending}
            onSelect={() => exit.mutate()}
          >
            <LogOutIcon aria-hidden />
            Exit
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <div className="flex shrink-0 items-center gap-0.5">
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label="Back"
          className="text-muted-foreground"
          disabled={!canGoBack}
          onClick={() => router.history.back()}
        >
          <ArrowLeftIcon aria-hidden />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label="Forward"
          className="text-muted-foreground"
          disabled={!canGoForward}
          onClick={() => router.history.forward()}
        >
          <ArrowRightIcon aria-hidden />
        </Button>
      </div>
      <nav
        aria-label="Main navigation"
        className="ml-2 flex shrink-0 items-center gap-1"
      >
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-xs text-muted-foreground aria-[current=page]:border-foreground/20 aria-[current=page]:text-foreground"
          aria-current={libraryActive ? "page" : undefined}
          onClick={onOpenLibrary}
        >
          <LibraryIcon aria-hidden className="size-3.5" />
          Library
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-xs text-muted-foreground"
          disabled
        >
          <CompassIcon aria-hidden className="size-3.5" />
          Discover
        </Button>
      </nav>
    </div>
  );
}
