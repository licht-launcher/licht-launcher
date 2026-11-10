import { useMutation } from "@tanstack/react-query";
import { useRouter, useRouterState } from "@tanstack/react-router";
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
import { openFeedback, quitApp } from "@/api/commands";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { SectionId } from "@/features/settings/model/sections";
import { friendlyError, messageOf } from "@/lib/errors";
import { cn } from "@/lib/utils";

import { launcherNavigationStyles } from "./launcher-navigation.styles";

export function LauncherNavigation({
  libraryActive,
  onOpenLibrary,
  discoverActive,
  onOpenDiscover,
  onOpenSettings,
}: {
  libraryActive: boolean;
  onOpenLibrary: () => void;
  discoverActive: boolean;
  onOpenDiscover: () => void;
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
  const openingSettings = useRef<SectionId | null>(null);
  const exit = useMutation({
    mutationFn: quitApp,
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });
  const feedback = useMutation({
    mutationFn: openFeedback,
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });

  function openSettings(section: SectionId) {
    openingSettings.current = section;
    setMenuOpen(false);
  }

  return (
    <div className={launcherNavigationStyles.root}>
      <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className={launcherNavigationStyles.brandTrigger}
          >
            Licht Launcher
            {menuOpen ? (
              <ChevronUpIcon
                aria-hidden
                className={launcherNavigationStyles.brandChevron}
              />
            ) : (
              <ChevronDownIcon
                aria-hidden
                className={launcherNavigationStyles.brandChevron}
              />
            )}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          aria-label="Licht menu"
          align="start"
          className={launcherNavigationStyles.menu}
          onCloseAutoFocus={(event) => {
            const section = openingSettings.current;
            if (section !== null) {
              event.preventDefault();
              openingSettings.current = null;
              onOpenSettings(section);
            }
          }}
        >
          <DropdownMenuItem onSelect={() => openSettings("account")}>
            <UsersIcon aria-hidden />
            Manage accounts
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => openSettings("account")}>
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
      <div className={launcherNavigationStyles.history}>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label="Back"
          className={launcherNavigationStyles.historyButton}
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
          className={launcherNavigationStyles.historyButton}
          disabled={!canGoForward}
          onClick={() => router.history.forward()}
        >
          <ArrowRightIcon aria-hidden />
        </Button>
      </div>
      <nav
        aria-label="Main navigation"
        className={launcherNavigationStyles.nav}
      >
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={cn(
            launcherNavigationStyles.navButton,
            launcherNavigationStyles.navButtonCurrent,
          )}
          aria-current={libraryActive ? "page" : undefined}
          onClick={onOpenLibrary}
        >
          <LibraryIcon
            aria-hidden
            className={launcherNavigationStyles.navIcon}
          />
          Library
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={cn(
            launcherNavigationStyles.navButton,
            launcherNavigationStyles.navButtonCurrent,
          )}
          aria-current={discoverActive ? "page" : undefined}
          onClick={onOpenDiscover}
        >
          <CompassIcon
            aria-hidden
            className={launcherNavigationStyles.navIcon}
          />
          Discover
        </Button>
      </nav>
    </div>
  );
}
